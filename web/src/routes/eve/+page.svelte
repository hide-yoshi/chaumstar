<script lang="ts">
	import { onMount } from 'svelte';
	import { listKeysets, postMint, postReview, ApiError } from '$lib/api';
	import { ensureWasm, mintFinish, mintStart, publishReview } from '$lib/wasm';
	import type { Credential, PublicKeyset, ReviewPayload } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';

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

	async function mintFreshCredential(ks: PublicKeyset): Promise<{ credential: Credential; ks: PublicKeyset }> {
		const issuedAt = new Date().toISOString().replace(/\.\d+/, '');
		const { state, request } = await mintStart(ks, ks.merchant_id, issuedAt);
		const response = await postMint(request);
		const credential = await mintFinish(state, response);
		return { credential, ks };
	}

	async function attackDoubleSpend(): Promise<void> {
		if (keysets.length === 0) return;
		busy = true;
		reset('attack: double-spend');
		try {
			await pushLine({ kind: 'cmd', text: 'silently mint a credential' });
			const { credential, ks } = await mintFreshCredential(keysets[0]);
			await pushLine({ kind: 'out', text: 'credential acquired', tone: 'ok' });
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

			await pushLine({ kind: 'cmd', text: 'first publish (legit)' });
			const p1 = await publishReview(credential, body);
			await postReview(p1);
			await pushLine({ kind: 'out', text: '201 created', tone: 'ok' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'second publish, same credential' });
			const p2 = await publishReview(credential, { ...body, text: '[eve] double-spend trial #2' });
			try {
				await postReview(p2);
				await pushLine({ kind: 'out', text: 'unexpected 201 — protocol BROKEN', tone: 'err' });
				await pushLine({ kind: 'stamp', text: 'ATTACK SUCCEEDED?!', tone: 'err' });
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} conflict (nullifier hpk already in spent_set)`,
					tone: 'ok'
				});
				await pushLine({ kind: 'stamp', text: 'attack rejected', tone: 'ok' });
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
			await pushLine({ kind: 'cmd', text: 'mint + publish (legit)' });
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
			const payload = await publishReview(credential, body);
			await pushLine({ kind: 'out', text: 'payload ready', tone: 'ok' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'tamper payload.review_body.text in-flight' });
			const tampered: ReviewPayload = JSON.parse(JSON.stringify(payload));
			tampered.review_body.text = '[eve] tampered: terrible service';
			await pushLine({ kind: 'out', text: 'text rewritten without re-signing', tone: 'warn' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'POST /api/v1/reviews' });
			try {
				await postReview(tampered);
				await pushLine({ kind: 'out', text: 'unexpected 201 — protocol BROKEN', tone: 'err' });
				await pushLine({ kind: 'stamp', text: 'ATTACK SUCCEEDED?!', tone: 'err' });
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} bad_request (ed25519 sig + BBS+ presentation_header both detected tamper)`,
					tone: 'ok'
				});
				await pushLine({ kind: 'stamp', text: 'attack rejected', tone: 'ok' });
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
			await pushLine({ kind: 'cmd', text: 'mint a real credential (then ruin the proof)' });
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
			const payload = await publishReview(credential, body);

			await pushLine({ kind: 'cmd', text: 'overwrite bbs_proof with random bytes' });
			const garbage = Array.from({ length: payload.credential_proof.bbs_proof.length / 2 }, () =>
				Math.floor(Math.random() * 256)
					.toString(16)
					.padStart(2, '0')
			).join('');
			const forged: ReviewPayload = JSON.parse(JSON.stringify(payload));
			forged.credential_proof.bbs_proof = garbage;
			await pushLine({ kind: 'out', text: 'proof randomized', tone: 'warn' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'POST /api/v1/reviews' });
			try {
				await postReview(forged);
				await pushLine({ kind: 'out', text: 'unexpected 201 — protocol BROKEN', tone: 'err' });
				await pushLine({ kind: 'stamp', text: 'ATTACK SUCCEEDED?!', tone: 'err' });
			} catch (e) {
				const status = e instanceof ApiError ? e.status : -1;
				await pushLine({
					kind: 'out',
					text: `${status} bad_request (bbs.proof.verify rejects the random proof)`,
					tone: 'ok'
				});
				await pushLine({ kind: 'stamp', text: 'attack rejected', tone: 'ok' });
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
		<p class="kicker">attacker / eve</p>
		<h1 class="title">attack console<span class="accent">.</span></h1>
		<p class="lede">
			three classes of attack the protocol claims to detect. each button runs the attack against
			the live server and shows the rejection (or, if something is broken, the success).
		</p>

		<div class="divider"></div>

		<div class="attack-grid">
			<button class="attack" disabled={busy} onclick={attackDoubleSpend}>
				<div class="kicker">attack 1</div>
				<div class="big">double-spend</div>
				<p class="sub">publish the same credential twice. expect 409 from the nullifier set.</p>
			</button>

			<button class="attack" disabled={busy} onclick={attackTamper}>
				<div class="kicker">attack 2</div>
				<div class="big">tamper</div>
				<p class="sub">
					rewrite review_body.text without re-signing. expect 400 from ed25519 + BBS+
					presentation_header.
				</p>
			</button>

			<button class="attack" disabled={busy} onclick={attackForge}>
				<div class="kicker">attack 3</div>
				<div class="big">forge</div>
				<p class="sub">
					replace bbs_proof with random bytes. expect 400 from bbs.proof.verify against PK_m.
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
	}
	.left {
		padding: 48px 56px 80px;
		max-width: 780px;
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
	.sub {
		font-size: 13px;
		color: var(--fg-muted);
	}
</style>
