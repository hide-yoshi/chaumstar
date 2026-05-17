<script lang="ts">
	import { onMount } from 'svelte';
	import { getRegistryKey, listKeysets, listReviews } from '$lib/api';
	import { ensureWasm, verifyInclusion, verifyProof, verifySth } from '$lib/wasm';
	import type {
		InclusionProof,
		ProductCategory,
		PublicKeyset,
		PurchaseTier,
		ReviewPayload,
		Sth
	} from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';
	import Term from '$lib/components/Term.svelte';

	type Status = 'pending' | 'verifying' | 'valid' | 'invalid';
	interface Row {
		payload: ReviewPayload;
		inclusion_proof: InclusionProof;
		status: Status;
		error?: string;
	}

	let keysets = $state<PublicKeyset[]>([]);
	let rows = $state<Row[]>([]);
	let sth = $state<Sth | null>(null);
	let registryPubkey = $state<string | null>(null);
	let selected = $state<number | null>(null);
	let termLines = $state<Line[]>([]);
	let termSubtitle = $state<string>('');
	let refreshing = $state<boolean>(false);
	let renderToken = 0;

	let filterTier = $state<PurchaseTier | 'any'>('any');
	let filterCategory = $state<ProductCategory | 'any'>('any');

	let filteredRows = $derived(
		rows.filter((r) => {
			const t = r.payload.credential_proof.purchase_tier;
			const c = r.payload.credential_proof.product_category;
			if (filterTier !== 'any' && t !== filterTier) return false;
			if (filterCategory !== 'any' && c !== filterCategory) return false;
			return true;
		})
	);

	onMount(async () => {
		await ensureWasm();
		await refresh();
	});

	async function refresh(): Promise<void> {
		refreshing = true;
		try {
			if (!registryPubkey) {
				const key = await getRegistryKey();
				registryPubkey = key.public_key;
			}
			keysets = await listKeysets();
			const list = await listReviews();
			sth = list.sth;
			rows = list.reviews.map((r) => ({
				payload: r.payload,
				inclusion_proof: r.inclusion_proof,
				status: 'pending' as Status
			}));
			for (let i = 0; i < rows.length; i++) {
				await verifyRow(i, false);
			}
			if (rows.length > 0) {
				selected = 0;
				renderTerminal(rows[0]);
			} else {
				selected = null;
				termLines = [];
				termSubtitle = '';
			}
		} finally {
			refreshing = false;
		}
	}

	function findKeyset(kid: string): PublicKeyset | undefined {
		return keysets.find((k) => k.keyset_id === kid);
	}

	async function verifyRow(i: number, render: boolean): Promise<void> {
		const row = rows[i];
		row.status = 'verifying';
		const ks = findKeyset(row.payload.credential_proof.keyset_id);
		if (!ks) {
			row.status = 'invalid';
			row.error = 'keyset not found';
			if (render) renderTerminal(row);
			return;
		}
		try {
			await verifyProof(row.payload, ks);
			if (sth && registryPubkey) {
				await verifySth(sth, registryPubkey);
				await verifyInclusion(row.payload, row.inclusion_proof, sth);
			}
			row.status = 'valid';
		} catch (e) {
			row.status = 'invalid';
			row.error = String(e);
		}
		if (render) renderTerminal(row);
	}

	async function renderTerminal(row: Row): Promise<void> {
		const token = ++renderToken;
		termSubtitle = `hpk ${shorten(row.payload.credential_proof.hpk)}`;
		termLines = [];
		const ok = row.status === 'valid';
		const t = row.payload.credential_proof.purchase_tier;
		const c = row.payload.credential_proof.product_category;
		const disclosedIndexes = ['0', '1'];
		if (t) disclosedIndexes.push('2');
		if (c) disclosedIndexes.push('3');
		const disclosedSummary =
			[t ? `tier=${t}` : null, c ? `category=${c}` : null].filter(Boolean).join(', ') ||
			'none';
		const sthSummary = sth
			? `root=${shorten(sth.root_hash, 6, 4)} size=${sth.tree_size}`
			: '<not loaded>';
		const script: Line[] = [
			{
				kind: 'cmd',
				text: `resolve_keyset(${row.payload.credential_proof.keyset_id})`,
				tip: '店ごとの公開鍵を取り出して、これからの検証に備える。'
			},
			{
				kind: 'out',
				text: 'PK_m loaded (BLS12-381 G2, 96 B)',
				tone: ok ? 'ok' : 'err',
				tip: '店の公開鍵をブラウザにロード完了。'
			},
			{ kind: 'hr' },
			{
				kind: 'cmd',
				text: `disclosed_indexes = [${disclosedIndexes.join(', ')}]`,
				tip: 'レビュアーが「読者に見せる」と決めた属性のインデックス。'
			},
			{
				kind: 'out',
				text: `disclosed: ${disclosedSummary}`,
				tone: 'warn',
				tip: '実際に開示された属性の値。非開示の属性も改ざんできない。'
			},
			{ kind: 'hr' },
			{
				kind: 'cmd',
				text: 'ed25519.verify(hpk, M_jcs, σ_ed)',
				tip: 'レビュー本文に対する署名を、レビュアーの公開鍵で検証する。'
			},
			{
				kind: 'out',
				text: ok ? 'holder signature valid' : 'holder signature failed',
				tone: ok ? 'ok' : 'err',
				tip: ok ? '本文が公開後に書き換えられていないことを確認。' : '本文と署名が一致しない。改ざんの疑い。'
			},
			{ kind: 'hr' },
			{
				kind: 'cmd',
				text: 'bbs.blind_proof.verify(π, PK_m, disclosed_msgs)',
				tip: 'ゼロ知識証明で「正当な credential を持つ客が書いた」ことを検証。誰かは特定されない。'
			},
			{
				kind: 'out',
				text: ok ? 'presentation proof valid' : 'presentation proof FAILED',
				tone: ok ? 'ok' : 'err',
				tip: ok ? '本物の購入客であることが暗号的に確認できた。' : '購入客であることを示せていない。偽造の疑い。'
			},
			{ kind: 'hr' },
			{
				kind: 'cmd',
				text: `transparency.sth = { ${sthSummary} }`,
				tip: 'Registry が「現在の公開ログ全体」に署名した状態 (STH) を取得。'
			},
			{
				kind: 'out',
				text: ok ? 'registry STH signature valid' : 'STH signature failed',
				tone: ok ? 'ok' : 'err',
				tip: ok ? 'Registry の署名 OK。STH の中身は本物。' : 'Registry の署名が壊れている。改ざんの疑い。'
			},
			{ kind: 'hr' },
			{
				kind: 'cmd',
				text: `merkle.inclusion.verify(leaf, path[${row.inclusion_proof.path.length}], root)`,
				tip: 'このレビューが公開ログに本当に含まれているかをハッシュ木で確認。'
			},
			{
				kind: 'out',
				text: ok ? 'inclusion proof valid' : 'inclusion proof FAILED',
				tone: ok ? 'ok' : 'err',
				tip: ok
					? '消されたり差し替えられたりしていない。Registry もこのレビューを「公開している」と認めている。'
					: '公開ログに含まれている証拠が破綻している。'
			},
			{
				kind: 'stamp',
				text: ok ? 'authentic' : 'rejected',
				tone: ok ? 'ok' : 'err',
				tip: ok ? 'すべての検証に合格。このレビューは本物。' : '検証に失敗。表示してよいレビューではない。'
			}
		];
		if (row.error) {
			script.push({ kind: 'hr' }, { kind: 'cmd', text: `error: ${row.error}` });
		}
		for (const line of script) {
			if (token !== renderToken) return;
			termLines = [...termLines, line];
			await delay(LINE_DELAY);
		}
	}

	function pickRow(originalIndex: number) {
		selected = originalIndex;
		renderTerminal(rows[originalIndex]);
	}

	function shorten(hex: string, head = 4, tail = 2): string {
		if (hex.length <= head + tail + 3) return hex;
		return `${hex.slice(0, head)}...${hex.slice(-tail)}`;
	}

	function stars(rating: number): string {
		return '★'.repeat(rating).padEnd(5, '☆');
	}

	function categoryEmoji(c: ProductCategory): string {
		return c === 'drinks' ? '☕' : c === 'food' ? '🥐' : '🛍';
	}

	function categoryLabel(c: ProductCategory): string {
		return c === 'drinks' ? 'ドリンク' : c === 'food' ? 'フード' : 'グッズ';
	}

	function tierJa(t: PurchaseTier): string {
		return t === 'low' ? '低' : t === 'mid' ? '中' : '高';
	}

	function anonHandle(hpk: string): string {
		return `匿名 #${hpk.slice(0, 4)}`;
	}

	function avatarHue(hpk: string): number {
		// derive a stable hue from the hpk (cosmetic only)
		let h = 0;
		for (let i = 0; i < Math.min(8, hpk.length); i++) {
			h = (h * 31 + hpk.charCodeAt(i)) % 360;
		}
		return h;
	}

	function formatDate(iso: string): string {
		// "2026-05-17T06:35:12Z" -> "2026/05/17"
		const m = iso.match(/(\d{4})-(\d{2})-(\d{2})/);
		return m ? `${m[1]}/${m[2]}/${m[3]}` : iso;
	}

	let validRows = $derived(rows.filter((r) => r.status === 'valid'));
	let avgRating = $derived(
		validRows.length === 0
			? 0
			: validRows.reduce((s, r) => s + r.payload.review_body.rating, 0) / validRows.length
	);
</script>

<main>
	<section class="left">
		<p class="kicker">reader / bob (読者)</p>
		<h1 class="title">reviews<span class="accent">.</span></h1>
		<p class="lede">
			公開レビュー一覧。各レビューに「本物の購入客が書いたか」と「Registry
			が後から消したり書き換えたりしていないか」を <Term
				tip="検証ロジックはサーバではなくブラウザの WASM 内で実行される。サーバの返した『verified』表示を信じる必要がない。"
				>あなたのブラウザだけで検証</Term
			>します。右の PROOF LOG に検証ステップが流れます。
		</p>

		<div class="divider"></div>

		{#if keysets.length === 0}
			<p class="kicker">loading…</p>
		{:else}
			{#each keysets as ks (ks.keyset_id)}
				<div class="store">
					<div class="store-thumb">{ks.merchant_id.charAt(0)}</div>
					<div class="store-info">
						<h2 class="store-name">{ks.merchant_id}</h2>
						<div class="store-issuer">by {ks.issuer_id}</div>
						<div class="store-rating">
							<span class="rating-stars" aria-label="{avgRating.toFixed(1)} stars">
								{#each [1, 2, 3, 4, 5] as i}
									<span class="star" class:filled={i <= Math.round(avgRating)}>★</span>
								{/each}
							</span>
							<span class="rating-num">{avgRating > 0 ? avgRating.toFixed(1) : '—'}</span>
							<span class="rating-count">{validRows.length}件のレビュー</span>
						</div>
					</div>
					<button class="store-refresh" disabled={refreshing} onclick={refresh} aria-label="refresh">
						{refreshing ? '…' : '↻'}
					</button>
				</div>
			{/each}
		{/if}

		<div class="filter-bar">
			<div class="filter-group">
				<span class="filter-label">カテゴリ</span>
				<button
					class="pill"
					class:active={filterCategory === 'any'}
					onclick={() => (filterCategory = 'any')}>すべて</button
				>
				<button
					class="pill"
					class:active={filterCategory === 'drinks'}
					onclick={() => (filterCategory = 'drinks')}>☕ ドリンク</button
				>
				<button
					class="pill"
					class:active={filterCategory === 'food'}
					onclick={() => (filterCategory = 'food')}>🥐 フード</button
				>
				<button
					class="pill"
					class:active={filterCategory === 'merch'}
					onclick={() => (filterCategory = 'merch')}>🛍 グッズ</button
				>
			</div>
			<div class="filter-group">
				<span class="filter-label">金額帯</span>
				<button
					class="pill"
					class:active={filterTier === 'any'}
					onclick={() => (filterTier = 'any')}>すべて</button
				>
				<button
					class="pill"
					class:active={filterTier === 'low'}
					onclick={() => (filterTier = 'low')}>低</button
				>
				<button
					class="pill"
					class:active={filterTier === 'mid'}
					onclick={() => (filterTier = 'mid')}>中</button
				>
				<button
					class="pill"
					class:active={filterTier === 'high'}
					onclick={() => (filterTier = 'high')}>高</button
				>
			</div>
			<span class="filter-count">{filteredRows.length} 件表示</span>
		</div>

		{#if rows.length === 0 && keysets.length > 0}
			<div class="empty-cta">
				<div class="kicker">no reviews yet · 初めての方へ</div>
				<p class="empty-lead">
					公開レビューがまだありません。下の順番で<strong>自分で発行→投稿→検証</strong>を一周してみてください。
				</p>
				<ol class="empty-steps">
					<li>
						<span class="step-no">01</span>
						<a class="step-link" href="/cafe/">cafe (店)</a>
						<span class="step-desc">で会計フォームを操作し、receipt を発行</span>
					</li>
					<li>
						<span class="step-no">02</span>
						<a class="step-link" href="/alice/">alice (レビュアー)</a>
						<span class="step-desc">で credential を claim → レビューを書いて publish</span>
					</li>
					<li>
						<span class="step-no">03</span>
						<span class="step-link active">bob (読者)</span>
						<span class="step-desc">に戻ってリロード → ここに verified バッジ付きで並ぶ</span>
					</li>
				</ol>
				<a class="btn primary" href="/cafe/">→ cafe からはじめる</a>
			</div>
		{/if}

		<div class="reviews">
			{#each filteredRows as row (row.payload.credential_proof.hpk)}
				{@const i = rows.indexOf(row)}
				{@const hpk = row.payload.credential_proof.hpk}
				{@const rating = row.payload.review_body.rating}
				{@const tier = row.payload.credential_proof.purchase_tier}
				{@const cat = row.payload.credential_proof.product_category}
				<button
					class="review-card"
					class:active={selected === i}
					class:invalid={row.status === 'invalid'}
					onclick={() => pickRow(i)}
				>
					<div class="rev-head">
						<span
							class="avatar"
							style="background: hsl({avatarHue(hpk)} 35% 30%); color: hsl({avatarHue(hpk)} 55% 80%);"
							aria-hidden="true">{hpk.slice(0, 1).toUpperCase()}</span
						>
						<div class="rev-who">
							<div class="rev-name">{anonHandle(hpk)}</div>
							<div class="rev-date">{formatDate(row.payload.review_body.timestamp)}</div>
						</div>
						<div class="rev-rating" aria-label="{rating} of 5">
							{#each [1, 2, 3, 4, 5] as i}
								<span class="star" class:filled={i <= rating}>★</span>
							{/each}
						</div>
					</div>
					<p class="rev-text">{row.payload.review_body.text}</p>
					<div class="rev-foot">
						<div class="rev-tags">
							{#if cat}
								<span class="tag">{categoryEmoji(cat)} {categoryLabel(cat)}</span>
							{/if}
							{#if tier}
								<span class="tag tier">金額帯 {tierJa(tier)}</span>
							{/if}
						</div>
						<span class="rev-verified" class:err={row.status === 'invalid'}>
							{#if row.status === 'valid'}
								<span class="vchk">✓</span> 認証済み
							{:else if row.status === 'invalid'}
								<span class="vchk">✗</span> 検証失敗
							{:else}
								検証中…
							{/if}
						</span>
					</div>
				</button>
			{/each}
		</div>

		{#if sth && registryPubkey}
			<details class="sth-foot">
				<summary>
					<Term
						tip="Registry が「現在この内容のレビュー集合が公開されている」と署名した状態 (Signed Tree Head)。1件でも消したり差し替えたりすると root_hash が変わり、各レビューの検証が落ちる。"
						>registry transparency log</Term
					>
					の現在の状態
				</summary>
				<div class="sth-body mono">
					<div><span class="k">STH root</span> <span class="v">{shorten(sth.root_hash, 12, 8)}</span></div>
					<div><span class="k">tree size</span> <span class="v">{sth.tree_size}</span></div>
					<div><span class="k">timestamp</span> <span class="v">{sth.timestamp}</span></div>
					<div><span class="k">registry pk</span> <span class="v">{shorten(registryPubkey, 12, 6)}</span></div>
				</div>
			</details>
		{/if}
	</section>

	<Terminal title="PROOF LOG" subtitle={termSubtitle} lines={termLines} />
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
	/* store header */
	.store {
		display: grid;
		grid-template-columns: 64px 1fr auto;
		gap: 16px;
		align-items: center;
		padding: 18px 20px;
		background: var(--bg-soft);
		border: 1px solid var(--border);
		border-radius: 4px;
		margin-bottom: 20px;
	}
	.store-thumb {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		background: linear-gradient(135deg, var(--accent) 0%, #c89236 100%);
		color: #1a1208;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 26px;
		font-weight: 700;
		font-family:
			-apple-system,
			BlinkMacSystemFont,
			'Inter',
			'Hiragino Sans',
			'Noto Sans JP',
			sans-serif;
	}
	.store-name {
		font-size: 20px;
		font-weight: 700;
		letter-spacing: -0.01em;
		color: var(--fg);
	}
	.store-issuer {
		font-size: 11px;
		color: var(--fg-dim);
		margin-top: 2px;
	}
	.store-rating {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 6px;
	}
	.rating-stars {
		display: inline-flex;
		gap: 1px;
	}
	.rating-stars .star {
		color: var(--fg-dim);
		font-size: 14px;
	}
	.rating-stars .star.filled {
		color: var(--accent);
	}
	.rating-num {
		font-size: 14px;
		color: var(--fg);
		font-weight: 600;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.rating-count {
		font-size: 11px;
		color: var(--fg-muted);
	}
	.store-refresh {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		border: 1px solid var(--border-strong);
		color: var(--fg-muted);
		font-size: 14px;
		transition: border-color 0.12s, color 0.12s;
	}
	.store-refresh:hover:not(:disabled) {
		border-color: var(--accent);
		color: var(--accent);
	}

	/* filter bar */
	.filter-bar {
		display: flex;
		flex-wrap: wrap;
		gap: 12px 18px;
		align-items: center;
		padding: 12px 14px;
		margin-bottom: 14px;
	}
	.filter-group {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.filter-label {
		font-size: 11px;
		color: var(--fg-dim);
		letter-spacing: 0.1em;
		margin-right: 4px;
	}
	.pill {
		padding: 5px 12px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		font-size: 12px;
		color: var(--fg-muted);
		background: transparent;
		transition: all 0.1s;
	}
	.pill:hover {
		border-color: var(--accent);
		color: var(--fg);
	}
	.pill.active {
		background: var(--accent);
		color: var(--bg-deepest);
		border-color: var(--accent);
		font-weight: 600;
	}
	.filter-count {
		margin-left: auto;
		font-size: 11px;
		color: var(--fg-dim);
	}

	/* review cards */
	.reviews {
		display: flex;
		flex-direction: column;
		gap: 14px;
		margin-bottom: 28px;
	}
	.review-card {
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 18px 20px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--bg-soft);
		cursor: pointer;
		transition: border-color 0.12s, transform 0.12s;
		font: inherit;
		color: inherit;
	}
	.review-card:hover {
		border-color: var(--accent);
		transform: translateY(-1px);
	}
	.review-card.active {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent);
	}
	.review-card.invalid {
		border-color: var(--err);
	}
	.rev-head {
		display: grid;
		grid-template-columns: 36px 1fr auto;
		gap: 12px;
		align-items: center;
	}
	.avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 13px;
		font-weight: 700;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.rev-who {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.rev-name {
		font-size: 13px;
		color: var(--fg);
		font-weight: 500;
	}
	.rev-date {
		font-size: 11px;
		color: var(--fg-dim);
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.rev-rating {
		display: inline-flex;
		gap: 1px;
	}
	.rev-rating .star {
		color: var(--fg-dim);
		font-size: 15px;
	}
	.rev-rating .star.filled {
		color: var(--accent);
	}
	.rev-text {
		font-size: 15px;
		color: var(--fg);
		line-height: 1.7;
		padding: 0 2px;
	}
	.rev-foot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
		flex-wrap: wrap;
	}
	.rev-tags {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}
	.tag {
		font-size: 11px;
		color: var(--fg-muted);
		border: 1px solid var(--border-strong);
		padding: 3px 8px;
		border-radius: 999px;
	}
	.tag.tier {
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.rev-verified {
		font-size: 11px;
		color: var(--ok);
		letter-spacing: 0.05em;
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.rev-verified.err {
		color: var(--err);
	}
	.vchk {
		font-weight: 700;
		font-size: 12px;
	}

	/* STH footer (collapsible) */
	.sth-foot {
		margin-top: 8px;
		padding: 12px 16px;
		background: var(--bg-soft);
		border: 1px solid var(--border);
		border-radius: 3px;
		font-size: 12px;
		color: var(--fg-muted);
	}
	.sth-foot summary {
		cursor: pointer;
		color: var(--fg-muted);
		font-size: 11px;
		letter-spacing: 0.05em;
		list-style-position: inside;
	}
	.sth-foot summary:hover {
		color: var(--fg);
	}
	.sth-body {
		margin-top: 10px;
		padding-top: 8px;
		border-top: 1px dashed var(--border);
		display: grid;
		gap: 4px;
		font-size: 11px;
	}
	.sth-body .k {
		color: var(--fg-dim);
		display: inline-block;
		min-width: 110px;
	}
	.sth-body .v {
		color: var(--accent);
	}
	.actions {
		margin-top: 18px;
	}
	strong {
		color: var(--accent);
	}
	.empty-cta {
		border: 1px solid var(--accent);
		background: var(--bg-soft);
		padding: 26px 28px;
		margin: 8px 0 24px;
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
	.empty-steps {
		list-style: none;
		padding: 0;
		margin: 0 0 22px;
	}
	.empty-steps li {
		display: flex;
		align-items: baseline;
		gap: 12px;
		padding: 10px 0;
		border-bottom: 1px solid var(--border);
		font-size: 13px;
	}
	.empty-steps li:last-child {
		border-bottom: none;
	}
	.step-no {
		color: var(--accent);
		font-size: 11px;
		letter-spacing: 0.1em;
		min-width: 24px;
	}
	.step-link {
		color: var(--accent);
		font-weight: 500;
		text-decoration: none;
		min-width: 130px;
	}
	.step-link.active {
		color: var(--fg-muted);
	}
	.step-desc {
		color: var(--fg-muted);
		font-size: 12px;
	}

	@media (max-width: 720px) {
		main {
			grid-template-columns: 1fr;
		}
		.left {
			padding: 24px 16px 32px;
			max-width: none;
		}
		.store {
			grid-template-columns: 48px 1fr auto;
			gap: 12px;
			padding: 14px 16px;
		}
		.store-thumb {
			width: 48px;
			height: 48px;
			font-size: 20px;
		}
		.store-name {
			font-size: 17px;
		}
		.filter-bar {
			padding: 8px 4px;
			gap: 10px 12px;
		}
		.pill {
			padding: 4px 9px;
			font-size: 11px;
		}
		.review-card {
			padding: 14px 16px;
		}
		.rev-head {
			grid-template-columns: 32px 1fr auto;
			gap: 10px;
		}
		.avatar {
			width: 32px;
			height: 32px;
			font-size: 12px;
		}
		.rev-rating .star {
			font-size: 13px;
		}
		.rev-text {
			font-size: 14px;
		}
		.rev-foot {
			gap: 8px;
		}
		.empty-cta {
			padding: 18px 16px;
		}
		.empty-steps li {
			flex-wrap: wrap;
		}
		.step-link {
			min-width: 0;
		}
	}
</style>
