<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { ApiError, listKeysets, postMint, postReview } from '$lib/api';
	import { ensureWasm, mintFinish, mintStart, publishReview } from '$lib/wasm';
	import type {
		Credential,
		MintContext,
		ProductCategory,
		PublicKeyset,
		PurchaseTier,
		ReviewBody,
		ReviewPayload
	} from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import Term from '$lib/components/Term.svelte';

	interface WalletEntry {
		id: number;
		credential: Credential;
		hpk: string;
		merchantId: string;
		issuedAt: string;
		purchaseTier: PurchaseTier;
		productCategory: ProductCategory;
		used: boolean;
	}

	interface PendingReceipt {
		keyset: PublicKeyset;
		tier: PurchaseTier;
		category: ProductCategory;
	}

	let keysets = $state<PublicKeyset[]>([]);
	let wallet = $state<WalletEntry[]>([]);
	let entryCounter = 0;
	let activeEntry = $state<number | null>(null);
	let pending = $state<PendingReceipt | null>(null);
	let reviewText = $state<string>('');
	let rating = $state<number>(5);
	let discloseTier = $state<boolean>(false);
	let discloseCategory = $state<boolean>(false);
	let busy = $state<boolean>(false);
	let termLines = $state<Line[]>([]);
	let termSubtitle = $state<string>('');
	let lastPublishedHpk = $state<string | null>(null);

	onMount(async () => {
		await ensureWasm();
		keysets = await listKeysets();
		readReceiptFromQuery();
	});

	function readReceiptFromQuery() {
		const q = page.url.searchParams;
		const merchant = q.get('merchant');
		const tier = q.get('tier') as PurchaseTier | null;
		const category = q.get('category') as ProductCategory | null;
		if (!merchant || !tier || !category) return;
		const ks = keysets.find((k) => k.merchant_id === merchant);
		if (!ks) return;
		pending = { keyset: ks, tier, category };
	}

	async function pushLine(line: Line): Promise<void> {
		termLines = [...termLines, line];
		await delay(LINE_DELAY);
	}

	function resetTerm(subtitle: string): void {
		termSubtitle = subtitle;
		termLines = [];
		lastPublishedHpk = null;
	}

	async function claim(receipt: PendingReceipt): Promise<void> {
		busy = true;
		resetTerm(`mint ← ${receipt.keyset.merchant_id} (${receipt.tier}/${receipt.category})`);
		try {
			const issuedAt = new Date().toISOString().replace(/\.\d+/, '');
			const ctx: MintContext = {
				merchant_id: receipt.keyset.merchant_id,
				issued_at: issuedAt,
				purchase_tier: receipt.tier,
				product_category: receipt.category
			};

			await pushLine({
				kind: 'cmd',
				text: `mintStart(keyset, {tier=${receipt.tier}, category=${receipt.category}})`,
				tip: 'ブラウザ内で秘密鍵 (holder key) を生成し、店に渡す commitment を作る。秘密鍵は外には出ない。'
			});
			const { state, request } = await mintStart(receipt.keyset, ctx);
			await pushLine({
				kind: 'out',
				text: 'commitment + holder keypair generated',
				tone: 'ok',
				tip: 'commitment と holder の鍵ペアを準備完了。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'POST /api/v1/mints',
				tip: '店に commitment と購入情報を送って blind signature をもらう。'
			});
			const response = await postMint(request);
			await pushLine({
				kind: 'out',
				text: '201 created (blind signature received)',
				tone: 'ok',
				tip: '店は中身を見ないまま署名を返してきた。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'mintFinish(state, response)',
				tip: 'ブラウザ側で blind を外して、実際に使える credential を組み立てる。'
			});
			const credential = await mintFinish(state, response);
			await pushLine({
				kind: 'out',
				text: 'blind signature unblinded and verified locally',
				tone: 'ok',
				tip: 'credential が完成。秘密は wallet 内に留まる。'
			});

			const entry: WalletEntry = {
				id: ++entryCounter,
				credential,
				hpk: credential.hpk,
				merchantId: credential.merchant_id,
				issuedAt: credential.issued_at,
				purchaseTier: credential.purchase_tier,
				productCategory: credential.product_category,
				used: false
			};
			wallet = [...wallet, entry];
			activeEntry = entry.id;
			pending = null;
			await pushLine({
				kind: 'stamp',
				text: 'credential stored',
				tone: 'ok',
				tip: 'credential を wallet に保管完了。これを使ってレビューを書ける。'
			});
		} catch (e) {
			await pushLine({ kind: 'out', text: String(e), tone: 'err' });
			await pushLine({ kind: 'stamp', text: 'mint failed', tone: 'err' });
		} finally {
			busy = false;
		}
	}

	function findEntry(id: number | null): WalletEntry | undefined {
		return id == null ? undefined : wallet.find((w) => w.id === id);
	}

	async function publish(): Promise<void> {
		const entry = findEntry(activeEntry);
		if (!entry || !reviewText.trim()) return;
		busy = true;
		resetTerm(`publish ← hpk ${shorten(entry.hpk)}`);
		try {
			const ts = new Date().toISOString().replace(/\.\d+/, '');
			const body: ReviewBody = {
				text: reviewText.trim(),
				rating,
				merchant_id: entry.merchantId,
				issuer_id: keysets.find((k) => k.merchant_id === entry.merchantId)?.issuer_id ?? '',
				issued_at: entry.issuedAt,
				timestamp: ts
			};
			const mask = {
				disclose_tier: discloseTier,
				disclose_category: discloseCategory
			};

			await pushLine({
				kind: 'cmd',
				text: `publishReview(credential, body, {tier=${discloseTier}, category=${discloseCategory}})`,
				tip: 'ブラウザで credential からゼロ知識証明を作り、本文への ed25519 署名も付ける。'
			});
			const payload: ReviewPayload = await publishReview(entry.credential, body, mask);
			await pushLine({
				kind: 'out',
				text: 'BBS+ presentation proof + ed25519 sig generated',
				tone: 'ok',
				tip: '「本物の客が書いた」証明と「本文の改ざんなし」署名が揃った。'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({
				kind: 'cmd',
				text: 'POST /api/v1/reviews',
				tip: 'サーバにレビューと証明を送って公開する。'
			});
			await postReview(payload);
			await pushLine({
				kind: 'out',
				text: '201 created (server verified and registered)',
				tone: 'ok',
				tip: 'サーバ側でも証明を検証して公開ログに登録された。'
			});
			entry.used = true;
			reviewText = '';
			discloseTier = false;
			discloseCategory = false;
			lastPublishedHpk = entry.hpk;
			await pushLine({
				kind: 'stamp',
				text: 'review published',
				tone: 'ok',
				tip: 'レビューを公開完了。bob で検証できる状態になった。'
			});
		} catch (e) {
			const msg = e instanceof ApiError ? `${e.status}: ${e.body}` : String(e);
			await pushLine({ kind: 'out', text: msg, tone: 'err' });
			await pushLine({ kind: 'stamp', text: 'publish failed', tone: 'err' });
		} finally {
			busy = false;
		}
	}

	function shorten(hex: string, head = 6, tail = 2): string {
		if (hex.length <= head + tail + 3) return hex;
		return `${hex.slice(0, head)}...${hex.slice(-tail)}`;
	}

	function categoryLabel(c: ProductCategory): string {
		return c === 'drinks' ? 'ドリンク' : c === 'food' ? 'フード' : 'グッズ';
	}

	function categoryEmoji(c: ProductCategory): string {
		return c === 'drinks' ? '☕' : c === 'food' ? '🥐' : '🛍';
	}

	function tierLabel(t: PurchaseTier): string {
		return t.toUpperCase();
	}

	function tierJa(t: PurchaseTier): string {
		return t === 'low' ? '低' : t === 'mid' ? '中' : '高';
	}
</script>

<main>
	<section class="left">
		<p class="kicker">reviewer / alice (レビュアー)</p>
		<h1 class="title">wallet<span class="accent">.</span></h1>
		<p class="lede">
			あなたの <Term tip="ブラウザのローカルストレージ。credential や秘密鍵を保持し、外部に漏れない。"
				>wallet</Term
			>。cafe から届いた receipt を
			<Term tip="receipt を実際の credential として手元に取り込む操作。"
				>claim</Term
			>
			すると、credential が手元に入る。本文を書いて <Term
				tip="credential から「本物の客であること」を示す proof を作り、レビュー本文と一緒にサーバへ送る操作。"
				>publish</Term
			> すれば、読者から検証可能なレビューになる。
		</p>

		<div class="divider"></div>

		{#if pending}
			<div class="incoming" in:fly={{ y: -28, duration: 520, easing: cubicOut }}>
				<div class="incoming-tag">incoming · 受け取ったレシート</div>
				<div class="paper">
					<div class="paper-merchant">{pending.keyset.merchant_id}</div>
					<div class="paper-issuer">issued by {pending.keyset.issuer_id}</div>
					<div class="paper-sep">─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─</div>
					<div class="paper-row">
						<span>category</span>
						<span>{categoryEmoji(pending.category)} {categoryLabel(pending.category)}</span>
					</div>
					<div class="paper-row">
						<span>tier</span>
						<span>{tierLabel(pending.tier)}</span>
					</div>
					<div class="paper-cut" aria-hidden="true">─ ─ ─ ─ ─ ─ ✂ ─ ─ ─ ─ ─ ─ ─ ─</div>
				</div>
				<button class="claim-btn" disabled={busy} onclick={() => claim(pending!)}>
					<span class="claim-arrow">→</span>
					<span class="claim-label">財布に入れる</span>
				</button>
			</div>
		{:else if wallet.length === 0}
			<div class="empty-cta">
				<div class="kicker">no pending receipt · 初めての方へ</div>
				<p class="empty-lead">
					まだ receipt を受け取っていません。<strong>cafe で会計フォーム</strong>を操作すると、ここに receipt が届きます。
				</p>
				<a class="btn primary" href="/cafe/">→ cafe で receipt を発行する</a>
			</div>
		{:else}
			<p class="kicker">
				no pending receipt. go to <a href="/cafe/" class="link">cafe</a> and "hand a receipt".
			</p>
		{/if}

		<div class="section-label">財布 · wallet</div>
		{#if wallet.length === 0}
			<div class="wallet-frame empty">
				<p class="wallet-empty-text">財布は空です。レシートを受け取って入れましょう。</p>
			</div>
		{:else}
			<div class="wallet-frame">
				<div class="wallet-stitch"></div>
				<div class="cards">
					{#each wallet as entry (entry.id)}
						<button
							class="card"
							class:active={activeEntry === entry.id}
							class:used={entry.used}
							onclick={() => (activeEntry = entry.id)}
						>
							<div class="card-head">
								<span class="card-merchant">{entry.merchantId}</span>
								<span class="card-tier" class:t-low={entry.purchaseTier === 'low'} class:t-mid={entry.purchaseTier === 'mid'} class:t-high={entry.purchaseTier === 'high'}>
									{tierLabel(entry.purchaseTier)}
								</span>
							</div>
							<div class="card-body">
								<span class="card-cat-emoji">{categoryEmoji(entry.productCategory)}</span>
								<span class="card-cat-label">{categoryLabel(entry.productCategory)}</span>
							</div>
							<div class="card-foot">
								<span class="card-hpk">{shorten(entry.hpk, 4, 4)}</span>
								<span class="card-stamp">{entry.used ? '使用済み' : '未使用'}</span>
							</div>
						</button>
					{/each}
				</div>
			</div>
		{/if}

			{#if findEntry(activeEntry) && !findEntry(activeEntry)!.used}
				<div class="composer">
					<div class="section-label">
						write review for {findEntry(activeEntry)!.merchantId}
					</div>
					<textarea
						bind:value={reviewText}
						placeholder="ハンドドリップが秀逸..."
						rows="4"
						disabled={busy}
					></textarea>
					<div class="row">
						<label>
							rating
							<select bind:value={rating} disabled={busy}>
								<option value={1}>★</option>
								<option value={2}>★★</option>
								<option value={3}>★★★</option>
								<option value={4}>★★★★</option>
								<option value={5}>★★★★★</option>
							</select>
						</label>
						<button class="btn primary" disabled={busy || !reviewText.trim()} onclick={publish}>
							publish
						</button>
					</div>
					<div class="disclosure">
						<div class="kicker">
							<Term
								tip="credential に署名された属性のうち、どれを読者に見せるかを選べる。開示しない属性も「値が credential に含まれていること」自体は検証可能 (= 改ざんできない)。"
								>選択的開示</Term
							> · disclose
						</div>
						<label>
							<input type="checkbox" bind:checked={discloseTier} disabled={busy} />
							<span>金額帯 (<code>{tierJa(findEntry(activeEntry)!.purchaseTier)}</code>)</span>
						</label>
						<label>
							<input type="checkbox" bind:checked={discloseCategory} disabled={busy} />
							<span
								>カテゴリ (<code
									>{categoryEmoji(findEntry(activeEntry)!.productCategory)}
									{categoryLabel(findEntry(activeEntry)!.productCategory)}</code
								>)</span
							>
						</label>
					</div>
				</div>
			{/if}

			{#if lastPublishedHpk}
				<a href="/bob/" class="published-link">
					<span class="kicker">just published</span>
					<span>→ see it on bob</span>
				</a>
			{/if}
	</section>

	<Terminal title="WALLET LOG" subtitle={termSubtitle} lines={termLines} />
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
	.link {
		color: var(--accent);
	}
	/* ── incoming receipt (thermal paper) ── */
	.incoming {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 14px;
		margin-bottom: 28px;
	}
	.incoming-tag {
		align-self: flex-start;
		font-size: 11px;
		letter-spacing: 0.2em;
		color: var(--fg-dim);
		text-transform: uppercase;
	}
	.paper {
		width: 320px;
		background: #f4ede0;
		color: #1a1410;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-size: 13px;
		line-height: 1.75;
		padding: 22px 24px 8px;
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
		position: relative;
	}
	.paper::before {
		content: '';
		position: absolute;
		left: 0;
		right: 0;
		top: 0;
		height: 6px;
		background: linear-gradient(180deg, rgba(0, 0, 0, 0.2) 0%, transparent 100%);
	}
	.paper-merchant {
		font-size: 16px;
		font-weight: 700;
		text-align: center;
		letter-spacing: 0.04em;
	}
	.paper-issuer {
		font-size: 10px;
		text-align: center;
		color: #5a4f44;
		letter-spacing: 0.06em;
		text-transform: lowercase;
		margin-top: 2px;
	}
	.paper-sep {
		text-align: center;
		color: #8a7a68;
		font-size: 12px;
		margin: 10px 0;
	}
	.paper-row {
		display: flex;
		justify-content: space-between;
		font-size: 12.5px;
	}
	.paper-row span:first-child {
		color: #5a4f44;
		text-transform: lowercase;
	}
	.paper-row span:last-child {
		color: #1a1410;
		font-weight: 500;
	}
	.paper-cta {
		text-align: center;
		font-weight: 600;
		font-size: 12.5px;
		color: #1a1410;
		margin-top: 4px;
	}
	.paper-cut {
		text-align: center;
		font-size: 12px;
		color: #8a7a68;
		padding: 8px 0 6px;
	}
	.claim-btn {
		padding: 14px 22px;
		font-size: 14px;
		font-weight: 700;
		letter-spacing: 0.15em;
		background: linear-gradient(180deg, #f4c074 0%, #d49a48 100%);
		color: #1a1208;
		border: 1px solid #c89236;
		border-radius: 3px;
		cursor: pointer;
		box-shadow:
			0 4px 0 #6b4f1f,
			0 5px 8px rgba(0, 0, 0, 0.3),
			inset 0 1px 0 rgba(255, 255, 255, 0.4);
		transition: transform 0.06s, box-shadow 0.06s;
		display: flex;
		align-items: center;
		gap: 10px;
		font-family: inherit;
	}
	.claim-btn:hover:not(:disabled) {
		filter: brightness(1.06);
	}
	.claim-btn:active:not(:disabled) {
		transform: translateY(3px);
		box-shadow:
			0 1px 0 #6b4f1f,
			0 3px 4px rgba(0, 0, 0, 0.3),
			inset 0 1px 0 rgba(255, 255, 255, 0.4);
	}
	.claim-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
		filter: grayscale(0.4);
	}
	.claim-arrow {
		font-size: 18px;
		line-height: 1;
	}

	/* ── wallet (billfold) ── */
	.wallet-frame {
		position: relative;
		background: linear-gradient(135deg, #3b2a1c 0%, #2a1d12 100%);
		border: 1px solid #1a120a;
		border-radius: 6px;
		padding: 24px 22px 22px;
		box-shadow:
			inset 0 2px 4px rgba(255, 255, 255, 0.04),
			inset 0 -2px 6px rgba(0, 0, 0, 0.5),
			0 6px 14px rgba(0, 0, 0, 0.35);
		margin-bottom: 18px;
	}
	.wallet-stitch {
		position: absolute;
		top: 8px;
		left: 8px;
		right: 8px;
		bottom: 8px;
		border: 1px dashed rgba(232, 178, 92, 0.25);
		border-radius: 4px;
		pointer-events: none;
	}
	.wallet-frame.empty {
		padding: 36px 22px;
		text-align: center;
	}
	.wallet-empty-text {
		color: rgba(240, 232, 212, 0.55);
		font-size: 13px;
		letter-spacing: 0.04em;
	}
	.cards {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		position: relative;
	}
	.card {
		position: relative;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		text-align: left;
		font: inherit;
		color: inherit;
		min-height: 140px;
		padding: 14px 16px;
		border: 1px solid #c89236;
		border-radius: 5px;
		background: linear-gradient(135deg, #f4dca0 0%, #e8b25c 60%, #c89236 100%);
		color: #2a1a08;
		cursor: pointer;
		box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
		transition: transform 0.12s, box-shadow 0.12s;
		overflow: hidden;
	}
	.card::before {
		content: '';
		position: absolute;
		inset: 4px;
		border: 1px dashed rgba(42, 26, 8, 0.18);
		border-radius: 3px;
		pointer-events: none;
	}
	.card:hover:not(.used) {
		transform: translateY(-2px);
		box-shadow: 0 6px 12px rgba(0, 0, 0, 0.35);
	}
	.card.active {
		outline: 2px solid #f4ede0;
		outline-offset: -3px;
	}
	.card.used {
		filter: grayscale(0.6) brightness(0.75);
		cursor: default;
	}
	.card-head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 8px;
		position: relative;
	}
	.card-merchant {
		font-size: 14px;
		font-weight: 700;
		letter-spacing: 0.02em;
	}
	.card-tier {
		font-size: 10px;
		letter-spacing: 0.15em;
		padding: 2px 8px;
		background: rgba(42, 26, 8, 0.85);
		color: #f4dca0;
		border-radius: 2px;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-weight: 600;
	}
	.card-body {
		display: flex;
		align-items: center;
		gap: 8px;
		position: relative;
	}
	.card-cat-emoji {
		font-size: 28px;
		line-height: 1;
	}
	.card-cat-label {
		font-size: 13px;
		font-weight: 500;
	}
	.card-foot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 10px;
		position: relative;
	}
	.card-hpk {
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		letter-spacing: 0.1em;
		color: rgba(42, 26, 8, 0.7);
	}
	.card-stamp {
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.15em;
		color: rgba(42, 26, 8, 0.85);
		border: 1px solid rgba(42, 26, 8, 0.5);
		padding: 2px 6px;
		border-radius: 2px;
	}
	.card.used .card-stamp {
		color: #d97861;
		border-color: #d97861;
		background: rgba(0, 0, 0, 0.15);
	}

	.composer {
		border: 1px solid var(--border);
		padding: 18px;
		margin-top: 14px;
		background: var(--bg-soft);
	}
	.composer textarea {
		margin-top: 8px;
	}
	.composer .row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 12px;
		gap: 12px;
	}
	.composer .row > label {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: 12px;
		color: var(--fg-muted);
	}
	.disclosure {
		margin-top: 16px;
		padding-top: 14px;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.disclosure label {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--fg);
		cursor: pointer;
	}
	.disclosure code {
		color: var(--accent);
	}
	.published-link {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 16px 20px;
		margin-top: 18px;
		border: 1px solid var(--accent);
		color: var(--accent);
		text-decoration: none;
		font-size: 14px;
		transition: background 0.12s;
	}
	.published-link:hover {
		background: var(--accent-soft);
	}
	code {
		background: var(--bg);
		padding: 1px 4px;
	}
	.empty-cta {
		border: 1px solid var(--accent);
		background: var(--bg-soft);
		padding: 26px 28px;
		margin: 8px 0 28px;
	}
	.empty-cta .empty-lead {
		font-size: 14px;
		color: var(--fg);
		line-height: 1.7;
		margin: 10px 0 18px;
	}
	.empty-cta .empty-lead strong {
		color: var(--accent);
	}

	@media (max-width: 720px) {
		main {
			grid-template-columns: 1fr;
		}
		.left {
			padding: 24px 16px 32px;
			max-width: none;
		}
		.paper {
			width: min(320px, calc(100vw - 56px));
		}
		.claim-btn {
			padding: 12px 18px;
			font-size: 13px;
			letter-spacing: 0.1em;
		}
		.wallet-frame {
			padding: 18px 14px 16px;
		}
		.cards {
			grid-template-columns: 1fr;
			gap: 10px;
		}
		.card {
			min-height: 120px;
		}
		.card-cat-emoji {
			font-size: 24px;
		}
		.composer {
			padding: 14px;
		}
		.composer .row {
			flex-direction: column;
			align-items: stretch;
			gap: 10px;
		}
	}
</style>
