<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError, listKeysets, postMint, postReview } from '$lib/api';
	import { ensureWasm, mintFinish, mintStart, publishReview } from '$lib/wasm';
	import type { Credential, MintContext, PublicKeyset, ReviewPayload } from '$lib/types';
	import { NO_DISCLOSURE } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';
	import Term from '$lib/components/Term.svelte';

	let keysets = $state<PublicKeyset[]>([]);
	let busy = $state<boolean>(false);
	let termSubtitle = $state<string>('attack console');
	let termLines = $state<Line[]>([]);

	onMount(async () => {
		await ensureWasm();
		keysets = await listKeysets();
	});

	async function pushLine(line: Line): Promise<void> {
		termLines = [...termLines, line];
		await delay(LINE_DELAY);
	}

	function reset(subtitle: string): void {
		termSubtitle = subtitle;
		termLines = [];
	}

	async function mintFreshCredential(
		ks: PublicKeyset
	): Promise<{ credential: Credential; ks: PublicKeyset }> {
		const issuedAt = new Date().toISOString().replace(/\.\d+/, '');
		const ctx: MintContext = {
			merchant_id: ks.merchant_id,
			issued_at: issuedAt,
			purchase_tier: 'mid',
			product_category: 'drinks'
		};
		const { state, request } = await mintStart(ks, ctx);
		const response = await postMint(request);
		const credential = await mintFinish(state, response);
		return { credential, ks };
	}

	async function attackDoubleSpend(): Promise<void> {
		if (keysets.length === 0) return;
		busy = true;
		reset('attack: double-spend');
		try {
			await pushLine({
				kind: 'cmd',
				text: 'silently mint a credential',
				tip: '攻撃用に通常通り credential を発行してもらう。'
			});
			const { credential, ks } = await mintFreshCredential(keysets[0]);
			await pushLine({
				kind: 'out',
				text: 'credential acquired',
				tone: 'ok',
				tip: '本物の credential を1つ手に入れた。'
			});
			await pushLine({ kind: 'hr' });

			const ts = new Date().toISOString().replace(/\.\d+/, '');
			const issuedAt = (credential as { issued_at: string }).issued_at;
			const body = {
				text: '[eve] double-spend trial #1',
				rating: 3,
				merchant_id: ks.merchant_id,
				issuer_id: ks.issuer_id,
				issued_at: issuedAt,
				timestamp: ts
			};

			await pushLine({
				kind: 'cmd',
				text: 'first publish (legit)',
				tip: '1回目: 正常にレビューを投稿する。'
			});
			const p1 = await publishReview(credential, body, NO_DISCLOSURE);
			await postReview(p1);
			await pushLine({
				kind: 'out',
				text: '201 created',
				tone: 'ok',
				tip: '1回目は成功。サーバは hpk を spent_set に登録する。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'second publish, same credential',
				tip: '2回目: 同じ credential を使って再度投稿を試みる (= 二重投稿)。'
			});
			const p2 = await publishReview(
				credential,
				{ ...body, text: '[eve] double-spend trial #2' },
				NO_DISCLOSURE
			);
			try {
				await postReview(p2);
				await pushLine({
					kind: 'out',
					text: 'unexpected 201 — protocol BROKEN',
					tone: 'err',
					tip: '想定外: サーバが二重投稿を許してしまった。プロトコル破綻の合図。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'ATTACK SUCCEEDED?!',
					tone: 'err',
					tip: '攻撃成功 = プロトコルが壊れている。'
				});
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} conflict (nullifier hpk already in spent_set)`,
					tone: 'ok',
					tip: '想定通り: 同じ hpk が既に使用済みリストにあったので拒否された。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'attack rejected',
					tone: 'ok',
					tip: '二重投稿は防げた。設計通り。'
				});
			}
		} catch (e) {
			await pushLine({ kind: 'out', text: String(e), tone: 'err' });
		} finally {
			busy = false;
		}
	}

	async function attackTamper(): Promise<void> {
		if (keysets.length === 0) return;
		busy = true;
		reset('attack: tamper');
		try {
			await pushLine({
				kind: 'cmd',
				text: 'mint + publish (legit)',
				tip: 'まず通常通り credential を発行して、有効な payload を作る。'
			});
			const { credential, ks } = await mintFreshCredential(keysets[0]);
			const issuedAt = (credential as { issued_at: string }).issued_at;
			const ts = new Date().toISOString().replace(/\.\d+/, '');
			const body = {
				text: 'great service',
				rating: 5,
				merchant_id: ks.merchant_id,
				issuer_id: ks.issuer_id,
				issued_at: issuedAt,
				timestamp: ts
			};
			const payload = await publishReview(credential, body, NO_DISCLOSURE);
			await pushLine({
				kind: 'out',
				text: 'payload ready',
				tone: 'ok',
				tip: '正規の payload (本文 + 署名 + 証明) が準備できた。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'tamper payload.review_body.text in-flight',
				tip: 'サーバに送る前に、本文だけ書き換える (署名は更新しない)。'
			});
			const tampered: ReviewPayload = JSON.parse(JSON.stringify(payload));
			tampered.review_body.text = '[eve] tampered: terrible service';
			await pushLine({
				kind: 'out',
				text: 'text rewritten without re-signing',
				tone: 'warn',
				tip: '本文が変わったが、署名と証明は元のまま。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'POST /api/v1/reviews',
				tip: '改ざんした payload をサーバに送信。'
			});
			try {
				await postReview(tampered);
				await pushLine({
					kind: 'out',
					text: 'unexpected 201 — protocol BROKEN',
					tone: 'err',
					tip: '想定外: サーバが改ざん本文を受け入れてしまった。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'ATTACK SUCCEEDED?!',
					tone: 'err',
					tip: '攻撃成功 = プロトコルが壊れている。'
				});
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} bad_request (ed25519 sig + BBS+ presentation_header both detected tamper)`,
					tone: 'ok',
					tip: '想定通り: 本文と署名が一致しないことをサーバが検出し拒否した。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'attack rejected',
					tone: 'ok',
					tip: '本文改ざんは防げた。設計通り。'
				});
			}
		} catch (e) {
			await pushLine({ kind: 'out', text: String(e), tone: 'err' });
		} finally {
			busy = false;
		}
	}

	async function attackForge(): Promise<void> {
		if (keysets.length === 0) return;
		busy = true;
		reset('attack: forge');
		try {
			await pushLine({
				kind: 'cmd',
				text: 'mint a real credential (then ruin the proof)',
				tip: '実在の credential を発行してから、証明部分だけ壊して投稿する。'
			});
			const { credential, ks } = await mintFreshCredential(keysets[0]);
			const issuedAt = (credential as { issued_at: string }).issued_at;
			const ts = new Date().toISOString().replace(/\.\d+/, '');
			const body = {
				text: '[eve] forged payload',
				rating: 5,
				merchant_id: ks.merchant_id,
				issuer_id: ks.issuer_id,
				issued_at: issuedAt,
				timestamp: ts
			};
			const payload = await publishReview(credential, body, NO_DISCLOSURE);

			await pushLine({
				kind: 'cmd',
				text: 'overwrite bbs_proof with random bytes',
				tip: 'ゼロ知識証明をランダムなバイト列で上書き (= 偽造)。'
			});
			const garbage = Array.from({ length: payload.credential_proof.bbs_proof.length / 2 }, () =>
				Math.floor(Math.random() * 256)
					.toString(16)
					.padStart(2, '0')
			).join('');
			const forged: ReviewPayload = JSON.parse(JSON.stringify(payload));
			forged.credential_proof.bbs_proof = garbage;
			await pushLine({
				kind: 'out',
				text: 'proof randomized',
				tone: 'warn',
				tip: '証明がデタラメ。これでサーバを騙せるか試す。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'POST /api/v1/reviews',
				tip: '偽造した payload をサーバに送信。'
			});
			try {
				await postReview(forged);
				await pushLine({
					kind: 'out',
					text: 'unexpected 201 — protocol BROKEN',
					tone: 'err',
					tip: '想定外: サーバが偽の証明を受け入れた。検証が壊れている。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'ATTACK SUCCEEDED?!',
					tone: 'err',
					tip: '攻撃成功 = プロトコルが壊れている。'
				});
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} bad_request (bbs.proof.verify rejects the random proof)`,
					tone: 'ok',
					tip: '想定通り: サーバが店の公開鍵で検証して、デタラメな証明を拒否した。'
				});
				await pushLine({
					kind: 'stamp',
					text: 'attack rejected',
					tone: 'ok',
					tip: '証明偽造は防げた。設計通り。'
				});
			}
		} catch (e) {
			await pushLine({ kind: 'out', text: String(e), tone: 'err' });
		} finally {
			busy = false;
		}
	}
</script>

<main>
	<section class="left">
		<p class="kicker">attacker / eve (攻撃者)</p>
		<h1 class="title">attack console<span class="accent">.</span></h1>
		<p class="lede">
			仕組みが防げると主張している3種類の攻撃を、本物のサーバに実際に投げて結果を確認します。すべて拒否されれば設計どおり。もし通ってしまったら仕組みが壊れている合図。
		</p>

		<div class="divider"></div>

		<div class="attack-grid">
			<button class="attack" disabled={busy} onclick={attackDoubleSpend}>
				<div class="kicker">attack 1</div>
				<div class="big">double-spend (二重投稿)</div>
				<p class="sub-ja">
					同じ credential で 2 回レビューを送る。同じ
					<Term tip="credential ごとに1つだけ存在する識別子。サーバが spent_set で重複を検出する (nullifier)。"
						>hpk</Term
					> が再び現れた瞬間に拒否される。
				</p>
			</button>

			<button class="attack" disabled={busy} onclick={attackTamper}>
				<div class="kicker">attack 2</div>
				<div class="big">tamper (本文改ざん)</div>
				<p class="sub-ja">
					レビュー本文だけ書き換えて送る。本文と署名が結びついているので、書き換えた瞬間に検証が通らなくなる。
				</p>
			</button>

			<button class="attack" disabled={busy} onclick={attackForge}>
				<div class="kicker">attack 3</div>
				<div class="big">forge (証明の偽造)</div>
				<p class="sub-ja">
					証明部分をデタラメなバイト列に差し替えて送る。サーバが店の公開鍵で検証して落ちるので拒否される。
				</p>
			</button>
		</div>
	</section>

	<Terminal title="ATTACK LOG" subtitle={termSubtitle} lines={termLines} />
</main>

<style>
	main {
		display: grid;
		grid-template-columns: 1fr 460px;
		min-height: calc(100vh - 84px);
		max-width: 1320px;
		margin: 0 auto;
	}
	.left {
		padding: 48px 56px 80px;
		max-width: 820px;
	}
	.attack-grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 14px;
	}
	.attack {
		text-align: left;
		font: inherit;
		color: inherit;
		background: var(--bg-soft);
		border: 1px solid var(--border);
		padding: 22px 26px;
		cursor: pointer;
		transition: border-color 0.12s;
	}
	.attack:hover:not(:disabled) {
		border-color: var(--accent);
	}
	.attack:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.big {
		font-size: 22px;
		font-weight: 600;
		margin: 4px 0 8px;
	}
	.sub-ja {
		margin-top: 8px;
		font-size: 13px;
		color: var(--fg-muted);
		line-height: 1.75;
	}

	@media (max-width: 720px) {
		main {
			grid-template-columns: 1fr;
		}
		.left {
			padding: 24px 16px 32px;
			max-width: none;
		}
		.attack {
			padding: 16px 18px;
		}
		.big {
			font-size: 18px;
		}
		.sub-ja {
			font-size: 12px;
		}
	}
</style>
