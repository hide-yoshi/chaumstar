<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { slide, fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { listKeysets } from '$lib/api';
	import type { ProductCategory, PublicKeyset } from '$lib/types';
	import { tierFromYen } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import Term from '$lib/components/Term.svelte';

	let keysets = $state<PublicKeyset[]>([]);
	let selectedKid = $state<string>('');
	let amountYen = $state<number>(500);
	let category = $state<ProductCategory>('drinks');
	let termSubtitle = $state<string>('');
	let termLines = $state<Line[]>([
		{ kind: 'cmd', text: 'GET /api/v1/keysets' },
		{ kind: 'out', text: 'fetching…', tone: 'warn' }
	]);

	let showReceipt = $state<boolean>(false);
	let receiptIssuedAt = $state<string>('');
	let receiptHpkPreview = $state<string>('');

	let tier = $derived(tierFromYen(amountYen));
	let selectedKeyset = $derived(keysets.find((k) => k.keyset_id === selectedKid));

	let issueHref = $derived(
		selectedKeyset
			? `/alice/?merchant=${encodeURIComponent(selectedKeyset.merchant_id)}` +
				`&tier=${tier}&category=${category}`
			: '#'
	);

	async function handReceipt() {
		if (!selectedKeyset || showReceipt) return;
		receiptIssuedAt = new Date().toISOString().slice(0, 19).replace('T', ' ');
		receiptHpkPreview = randomHpkPreview();
		showReceipt = true;
		// allow user ~2.2s to register the printout, then navigate
		await sleep(2200);
		await goto(issueHref);
	}

	function skipAnimation() {
		if (!showReceipt) return;
		goto(issueHref);
	}

	function sleep(ms: number): Promise<void> {
		return new Promise((r) => setTimeout(r, ms));
	}

	function randomHpkPreview(): string {
		// purely cosmetic — the real hpk is generated client-side after navigation
		const hex = '0123456789abcdef';
		let out = '';
		for (let i = 0; i < 6; i++) out += hex[Math.floor(Math.random() * 16)];
		return out + '...' + hex[Math.floor(Math.random() * 16)] + hex[Math.floor(Math.random() * 16)];
	}

	function addAmount(delta: number) {
		amountYen = Math.max(0, (amountYen || 0) + delta);
	}

	function clearAmount() {
		amountYen = 0;
	}

	function pickCategory(c: ProductCategory) {
		category = c;
	}

	function categoryLabel(c: ProductCategory): string {
		return c === 'drinks' ? 'ドリンク' : c === 'food' ? 'フード' : 'グッズ';
	}

	function categoryEmoji(c: ProductCategory): string {
		return c === 'drinks' ? '☕' : c === 'food' ? '🥐' : '🛍';
	}

	onMount(async () => {
		keysets = await listKeysets();
		if (keysets.length > 0) selectedKid = keysets[0].keyset_id;
		termSubtitle = `${keysets.length} keyset${keysets.length === 1 ? '' : 's'}`;
		termLines = [
			{
				kind: 'cmd',
				text: 'GET /api/v1/keysets',
				tip: 'サーバから店の公開鍵セット一覧を取得する。'
			},
			{
				kind: 'out',
				text: `200 ok · ${keysets.length} record(s)`,
				tone: 'ok',
				tip: '取得に成功。下に1件ずつ展開。'
			},
			{ kind: 'hr' },
			...keysets.flatMap((ks) => [
				{
					kind: 'cmd' as const,
					text: `kid_${ks.keyset_id}`,
					tip: 'keyset の識別子 (key id)。店ごとに1つ。'
				},
				{
					kind: 'out' as const,
					text: `${ks.issuer_id} / ${ks.merchant_id}`,
					tone: 'ok' as const,
					tip: 'この keyset を持つ「発行元 / 店舗」のペア。'
				},
				{
					kind: 'out' as const,
					text: `PK_m = ${shorten(ks.public_key_bytes)}`,
					tone: 'warn' as const,
					tip: 'この店の公開鍵 (PK_m)。読者がレビューを検証する時に使う。秘密鍵はサーバ側だけが持つ。'
				}
			])
		];
	});

	function shorten(hex: string, head = 12, tail = 4): string {
		if (hex.length <= head + tail + 3) return hex;
		return `${hex.slice(0, head)}...${hex.slice(-tail)}`;
	}
</script>

<main>
	<section class="left">
		<p class="kicker">issuer / cafe (店)</p>
		<h1 class="title">checkout counter<span class="accent">.</span></h1>
		<p class="lede">
			お店のカウンター。会計フォームに金額とカテゴリを入れて「<strong>レシートを渡す</strong>」を押すと、レシートが
			alice の <Term tip="ブラウザのローカルストレージ。credential や秘密鍵を保持し、外部に漏れない。"
				>wallet</Term
			> に届きます。
		</p>

		<div class="divider"></div>

		{#if keysets.length === 0}
			<p class="kicker">loading…</p>
		{:else}
			<div class="section-label">会計</div>
			<div class="cashier">
				<div class="display" aria-label="customer display">
					<div class="display-top">
						<span class="display-tag">MERCHANT</span>
						<span class="display-merchant">{selectedKeyset?.merchant_id ?? '—'}</span>
					</div>
					<div class="display-amount">
						<span class="yen">¥</span><span class="amt-num">{amountYen.toLocaleString()}</span>
					</div>
					<div class="display-bottom">
						<span>{categoryEmoji(category)} {categoryLabel(category)}</span>
						<span>tier · {tier}</span>
					</div>
				</div>

				<div class="pad-group">
					<span class="pad-label">category</span>
					<div class="cat-grid">
						<button
							class="cat-btn"
							class:active={category === 'drinks'}
							onclick={() => pickCategory('drinks')}
						>
							<span class="cat-glyph">☕</span>
							<span class="cat-name">ドリンク</span>
						</button>
						<button
							class="cat-btn"
							class:active={category === 'food'}
							onclick={() => pickCategory('food')}
						>
							<span class="cat-glyph">🥐</span>
							<span class="cat-name">フード</span>
						</button>
						<button
							class="cat-btn"
							class:active={category === 'merch'}
							onclick={() => pickCategory('merch')}
						>
							<span class="cat-glyph">🛍</span>
							<span class="cat-name">グッズ</span>
						</button>
					</div>
				</div>

				<div class="pad-group">
					<span class="pad-label">amount</span>
					<div class="amount-row">
						<button class="qa-btn" onclick={() => addAmount(100)}>+100</button>
						<button class="qa-btn" onclick={() => addAmount(500)}>+500</button>
						<button class="qa-btn" onclick={() => addAmount(1000)}>+1k</button>
						<button class="qa-btn" onclick={() => addAmount(5000)}>+5k</button>
						<button class="qa-btn clear" onclick={clearAmount}>CLR</button>
					</div>
					<input
						class="amount-input"
						type="number"
						min="0"
						step="100"
						bind:value={amountYen}
						aria-label="amount in yen"
					/>
				</div>

				<div class="pad-group">
					<span class="pad-label">tier (自動判定)</span>
					<div class="tier-row">
						<span class="tier-pip" class:active={tier === 'low'}>LOW · ¥0–999</span>
						<span class="tier-pip" class:active={tier === 'mid'}>MID · ¥1k–4.9k</span>
						<span class="tier-pip" class:active={tier === 'high'}>HIGH · ¥5k+</span>
					</div>
				</div>

				{#if keysets.length > 1}
					<label class="keyset-pick">
						<span>keyset</span>
						<select bind:value={selectedKid}>
							{#each keysets as ks (ks.keyset_id)}
								<option value={ks.keyset_id}>{ks.merchant_id}</option>
							{/each}
						</select>
					</label>
				{/if}

				<button
					class="tender-btn"
					onclick={handReceipt}
					disabled={!selectedKeyset || showReceipt || amountYen <= 0}
				>
					<span class="tender-arrow">→</span>
					<span class="tender-label">レシートを渡す</span>
				</button>
			</div>

			<div class="note">
				<div class="kicker">handoff</div>
				<p class="note-body">
					客が会計すると、お店は<strong>金額帯とカテゴリ</strong>だけを記録した
					<Term
						tip="店から客への「会計の証拠」。秘密のままレビューに紐づけられる。中身は店が見れない blind signature。"
						>credential</Term
					>
					を発行する。<strong>客が誰かは店から見えない</strong>。詳しい暗号フローは右の
					ISSUER STATE ログを参照。
				</p>
			</div>

			<div class="section-label">registered keysets</div>
			{#each keysets as ks (ks.keyset_id)}
				<div class="card">
					<div class="row">
						<div>
							<div class="kicker">issuer</div>
							<div class="big">{ks.issuer_id}</div>
						</div>
						<div>
							<div class="kicker">merchant</div>
							<div class="big">{ks.merchant_id}</div>
						</div>
					</div>
					<div class="meta">
						<span>kid_{ks.keyset_id}</span>
						<span>·</span>
						<span>PK_m {shorten(ks.public_key_bytes)}</span>
					</div>
				</div>
			{/each}
		{/if}
	</section>

	<Terminal title="ISSUER STATE" subtitle={termSubtitle} lines={termLines} />
</main>

{#if showReceipt}
	<div
		class="receipt-overlay"
		transition:fade={{ duration: 220 }}
		onclick={skipAnimation}
		onkeydown={(e) => e.key === 'Enter' && skipAnimation()}
		role="button"
		tabindex="-1"
	>
		<div class="printer">
			<div class="printer-body">
				<div class="printer-led"></div>
				<span class="printer-label">CHAUMSTAR · MINT PRINTER</span>
			</div>
			<div class="printer-slot"></div>
		</div>
		<div
			class="receipt-paper"
			transition:slide={{ duration: 1100, easing: cubicOut }}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="presentation"
		>
			<div class="r-inner">
				<div class="r-merchant">{selectedKeyset?.merchant_id ?? '—'}</div>
				<div class="r-issuer">issued by {selectedKeyset?.issuer_id ?? '—'}</div>
				<div class="r-sep">─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─</div>
				<div class="r-row"><span>amount</span><span>¥{amountYen.toLocaleString()}</span></div>
				<div class="r-row">
					<span>category</span>
					<span>{categoryEmoji(category)} {categoryLabel(category)}</span>
				</div>
				<div class="r-row"><span>tier</span><span>{tier}</span></div>
				<div class="r-row"><span>issued</span><span>{receiptIssuedAt}</span></div>
				<div class="r-sep">─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─</div>
				<div class="r-cta">→ claim on alice</div>
				<div class="r-note">hpk preview {receiptHpkPreview} (wallet で生成)</div>
			</div>
			<div class="r-cut" aria-hidden="true">
				<span>─ ─ ─ ─ ─ ─ ✂ ─ ─ ─ ─ ─ ─ ─ ─</span>
			</div>
		</div>
		<p class="overlay-hint">クリックでスキップ · まもなく alice へ移動します</p>
	</div>
{/if}

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
	.card {
		border: 1px solid var(--border);
		padding: 18px 22px;
		margin-bottom: 14px;
		background: var(--bg-soft);
	}
	.row {
		display: flex;
		gap: 48px;
	}
	.big {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
		color: var(--fg);
		margin-top: 4px;
	}
	.meta {
		margin-top: 14px;
		font-size: 11px;
		color: var(--fg-dim);
		display: flex;
		gap: 10px;
	}
	.cashier {
		display: flex;
		flex-direction: column;
		gap: 14px;
		padding: 22px;
		background: var(--bg-soft);
		border: 1px solid var(--border);
		border-radius: 4px;
		margin-bottom: 18px;
	}

	.display {
		background: linear-gradient(180deg, #0d1210 0%, #050807 100%);
		color: var(--accent);
		padding: 16px 20px;
		border: 1px solid #1a1f1c;
		border-radius: 3px;
		box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.6);
		display: grid;
		gap: 4px;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		position: relative;
		overflow: hidden;
	}
	.display::before {
		content: '';
		position: absolute;
		inset: 0;
		background: repeating-linear-gradient(
			180deg,
			transparent 0px,
			transparent 2px,
			rgba(232, 178, 92, 0.05) 3px
		);
		pointer-events: none;
	}
	.display-top {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.display-tag {
		font-size: 9px;
		letter-spacing: 0.28em;
		color: rgba(232, 178, 92, 0.55);
	}
	.display-merchant {
		font-size: 13px;
		color: var(--accent);
	}
	.display-amount {
		font-size: 46px;
		font-weight: 700;
		text-align: right;
		letter-spacing: 0.04em;
		line-height: 1;
		margin: 10px 0 6px;
		text-shadow: 0 0 10px rgba(232, 178, 92, 0.35);
	}
	.display-amount .yen {
		font-size: 26px;
		margin-right: 6px;
		opacity: 0.85;
	}
	.display-bottom {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
		color: rgba(232, 178, 92, 0.7);
		letter-spacing: 0.15em;
		text-transform: uppercase;
	}

	.pad-group {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.pad-label {
		font-size: 10px;
		letter-spacing: 0.2em;
		color: var(--fg-dim);
		text-transform: uppercase;
	}

	.cat-grid {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: 8px;
	}
	.cat-btn {
		padding: 14px 12px;
		border: 1px solid var(--border-strong);
		background: var(--bg);
		color: var(--fg);
		cursor: pointer;
		text-align: center;
		border-radius: 3px;
		transition: transform 0.06s, background 0.1s, border-color 0.1s;
		box-shadow: 0 2px 0 rgba(0, 0, 0, 0.3);
		display: flex;
		flex-direction: column;
		gap: 4px;
		align-items: center;
		font: inherit;
	}
	.cat-btn:hover:not(.active) {
		border-color: var(--accent);
		background: rgba(232, 178, 92, 0.05);
	}
	.cat-btn.active {
		background: var(--accent);
		color: var(--bg-deepest);
		border-color: var(--accent);
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
		transform: translateY(1px);
	}
	.cat-glyph {
		font-size: 24px;
		line-height: 1;
		filter: grayscale(0);
	}
	.cat-btn.active .cat-glyph {
		filter: none;
	}
	.cat-name {
		font-size: 11px;
		letter-spacing: 0.15em;
		text-transform: uppercase;
		font-weight: 500;
	}

	.amount-row {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 6px;
		margin-bottom: 6px;
	}
	.qa-btn {
		padding: 10px 0;
		border: 1px solid var(--border-strong);
		background: var(--bg);
		color: var(--fg);
		cursor: pointer;
		border-radius: 2px;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-size: 12px;
		letter-spacing: 0.04em;
		transition: transform 0.06s, color 0.1s, border-color 0.1s;
		box-shadow: 0 2px 0 rgba(0, 0, 0, 0.3);
	}
	.qa-btn:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.qa-btn:active {
		transform: translateY(1px);
		box-shadow: 0 1px 0 rgba(0, 0, 0, 0.3);
	}
	.qa-btn.clear {
		border-color: rgba(217, 120, 97, 0.5);
		color: var(--err);
	}
	.qa-btn.clear:hover {
		border-color: var(--err);
	}
	.amount-input {
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-size: 16px;
		text-align: right;
		padding: 10px 14px;
		width: 100%;
	}

	.tier-row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: 6px;
	}
	.tier-pip {
		padding: 8px 10px;
		text-align: center;
		border: 1px solid var(--border);
		font-size: 10px;
		letter-spacing: 0.1em;
		color: var(--fg-dim);
		text-transform: uppercase;
		border-radius: 2px;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		background: var(--bg);
	}
	.tier-pip.active {
		border-color: var(--accent);
		color: var(--accent);
		background: rgba(232, 178, 92, 0.08);
		box-shadow: 0 0 0 1px rgba(232, 178, 92, 0.2);
	}

	.keyset-pick {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 10px;
		color: var(--fg-muted);
		letter-spacing: 0.2em;
		text-transform: uppercase;
	}
	.keyset-pick select {
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		flex: 1;
	}

	.tender-btn {
		margin-top: 6px;
		padding: 18px 22px;
		font-size: 14px;
		font-weight: 700;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		background: linear-gradient(180deg, #f4c074 0%, #d49a48 100%);
		color: #1a1208;
		border: 1px solid #c89236;
		border-radius: 3px;
		cursor: pointer;
		box-shadow:
			0 4px 0 #6b4f1f,
			0 6px 8px rgba(0, 0, 0, 0.35),
			inset 0 1px 0 rgba(255, 255, 255, 0.4);
		transition: transform 0.06s, box-shadow 0.06s;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 14px;
		font-family: inherit;
	}
	.tender-btn:hover:not(:disabled) {
		filter: brightness(1.06);
	}
	.tender-btn:active:not(:disabled) {
		transform: translateY(3px);
		box-shadow:
			0 1px 0 #6b4f1f,
			0 3px 4px rgba(0, 0, 0, 0.3),
			inset 0 1px 0 rgba(255, 255, 255, 0.4);
	}
	.tender-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
		filter: grayscale(0.4);
	}
	.tender-arrow {
		font-size: 20px;
		line-height: 1;
	}
	.tender-label {
		letter-spacing: 0.12em;
	}
	.note {
		margin-top: 36px;
		border-left: 2px solid var(--accent);
		padding: 14px 22px;
		background: var(--bg-soft);
		border-radius: 2px;
	}
	.note-body {
		margin-top: 8px;
		font-size: 13px;
		color: var(--fg-muted);
		line-height: 1.75;
	}
	.note-body strong {
		color: var(--fg);
	}
	code {
		color: var(--accent);
		background: var(--bg);
		padding: 1px 4px;
		font-size: 0.95em;
	}

	/* ─── receipt printer overlay ─────────────────────────── */
	.receipt-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.72);
		backdrop-filter: blur(2px);
		z-index: 1000;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding-top: 64px;
		overflow-y: auto;
		cursor: pointer;
	}
	.printer {
		width: 360px;
		display: flex;
		flex-direction: column;
		align-items: stretch;
	}
	.printer-body {
		background: linear-gradient(180deg, #2a2a2a 0%, #1a1a1a 100%);
		border: 1px solid #3a3a3a;
		border-bottom: none;
		padding: 12px 20px;
		display: flex;
		align-items: center;
		gap: 12px;
		box-shadow: 0 -2px 0 #4a4a4a inset;
		border-top-left-radius: 4px;
		border-top-right-radius: 4px;
	}
	.printer-led {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--ok);
		box-shadow: 0 0 8px var(--ok);
		animation: led-blink 0.7s ease-in-out infinite alternate;
	}
	.printer-label {
		font-size: 10px;
		letter-spacing: 0.2em;
		color: #888;
		text-transform: uppercase;
	}
	@keyframes led-blink {
		from {
			opacity: 0.4;
		}
		to {
			opacity: 1;
		}
	}
	.printer-slot {
		height: 14px;
		background: #0a0a0a;
		border: 1px solid #3a3a3a;
		border-top: none;
		position: relative;
		box-shadow: inset 0 4px 6px rgba(0, 0, 0, 0.9);
	}
	.printer-slot::after {
		content: '';
		position: absolute;
		left: 12px;
		right: 12px;
		top: 50%;
		height: 1px;
		background: #1f1f1f;
	}
	.receipt-paper {
		width: 320px;
		background: #f4ede0;
		color: #1a1410;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-size: 13px;
		line-height: 1.7;
		box-shadow: 0 12px 24px rgba(0, 0, 0, 0.5);
		cursor: default;
		overflow: hidden;
		animation: paper-jitter 0.18s steps(2) 0s 6;
	}
	@keyframes paper-jitter {
		0% {
			transform: translateX(0);
		}
		50% {
			transform: translateX(-1px);
		}
		100% {
			transform: translateX(1px);
		}
	}
	.r-inner {
		padding: 22px 22px 6px;
	}
	.r-merchant {
		font-size: 16px;
		font-weight: 700;
		text-align: center;
		letter-spacing: 0.04em;
	}
	.r-issuer {
		font-size: 10px;
		text-align: center;
		color: #5a4f44;
		letter-spacing: 0.06em;
		text-transform: lowercase;
		margin-top: 2px;
	}
	.r-sep {
		text-align: center;
		color: #8a7a68;
		font-size: 12px;
		letter-spacing: 0.1em;
		margin: 10px 0;
	}
	.r-row {
		display: flex;
		justify-content: space-between;
		font-size: 12.5px;
	}
	.r-row span:first-child {
		color: #5a4f44;
		text-transform: lowercase;
	}
	.r-row span:last-child {
		color: #1a1410;
		font-weight: 500;
	}
	.r-cta {
		text-align: center;
		font-weight: 600;
		font-size: 12.5px;
		color: #1a1410;
		margin-top: 4px;
	}
	.r-note {
		font-size: 9px;
		color: #8a7a68;
		text-align: center;
		margin-top: 8px;
		letter-spacing: 0.04em;
	}
	.r-cut {
		text-align: center;
		font-size: 12px;
		color: #8a7a68;
		letter-spacing: 0.05em;
		padding: 6px 0 8px;
		background:
			linear-gradient(#f4ede0, #f4ede0),
			linear-gradient(135deg, #f4ede0 25%, transparent 25%) -8px 0,
			linear-gradient(225deg, #f4ede0 25%, transparent 25%) -8px 0;
	}
	.overlay-hint {
		color: var(--fg-dim);
		font-size: 11px;
		letter-spacing: 0.1em;
		margin-top: 18px;
		text-transform: uppercase;
	}

	@media (prefers-reduced-motion: reduce) {
		.receipt-paper {
			animation: none;
		}
		.printer-led {
			animation: none;
		}
	}

	@media (max-width: 720px) {
		main {
			grid-template-columns: 1fr;
		}
		.left {
			padding: 24px 16px 32px;
			max-width: none;
		}
		.cashier {
			padding: 14px;
		}
		.display {
			padding: 12px 14px;
		}
		.display-amount {
			font-size: 36px;
		}
		.display-amount .yen {
			font-size: 22px;
		}
		.cat-btn {
			padding: 10px 6px;
		}
		.cat-glyph {
			font-size: 20px;
		}
		.cat-name {
			font-size: 10px;
		}
		.amount-row {
			grid-template-columns: repeat(3, 1fr);
		}
		.tier-pip {
			font-size: 9px;
			padding: 6px 4px;
		}
		.tender-btn {
			padding: 14px;
			font-size: 12px;
			letter-spacing: 0.12em;
		}
		.card,
		.note {
			padding: 14px 16px;
		}
		.row {
			gap: 18px;
		}
		.printer,
		.receipt-paper {
			width: min(320px, calc(100vw - 32px));
		}
		.receipt-overlay {
			padding-top: 24px;
		}
	}
</style>
