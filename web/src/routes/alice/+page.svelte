<script lang="ts">
	import { onMount } from 'svelte';
	import { listKeysets, postMint, postReview, ApiError } from '$lib/api';
	import { ensureWasm, mintStart, mintFinish, publishReview } from '$lib/wasm';
	import type { Credential, PublicKeyset, ReviewBody, ReviewPayload } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';

	interface WalletEntry {
		id: number;
		credential: Credential;
		hpk: string;
		merchantId: string;
		issuedAt: string;
		used: boolean;
	}

	let keysets = $state<PublicKeyset[]>([]);
	let wallet = $state<WalletEntry[]>([]);
	let entryCounter = 0;
	let activeEntry = $state<number | null>(null);
	let reviewText = $state<string>('');
	let rating = $state<number>(5);
	let busy = $state<boolean>(false);
	let termLines = $state<Line[]>([]);
	let termSubtitle = $state<string>('');
	let lastPublishedHpk = $state<string | null>(null);

	onMount(async () => {
		await ensureWasm();
		keysets = await listKeysets();
	});

	async function pushLine(line: Line): Promise<void> {
		termLines = [...termLines, line];
		await delay(LINE_DELAY);
	}

	function resetTerm(subtitle: string): void {
		termSubtitle = subtitle;
		termLines = [];
		lastPublishedHpk = null;
	}

	async function mint(keyset: PublicKeyset): Promise<void> {
		busy = true;
		resetTerm(`mint ← ${keyset.merchant_id}`);
		try {
			const issuedAt = new Date().toISOString().replace(/\.\d+/, '');
			await pushLine({
				kind: 'cmd',
				text: `mintStart(keyset, "${keyset.merchant_id}", "${issuedAt}")`
			});
			const { state, request } = await mintStart(keyset, keyset.merchant_id, issuedAt);
			await pushLine({ kind: 'out', text: 'commitment + holder keypair generated', tone: 'ok' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'POST /api/v1/mints' });
			const response = await postMint(request);
			await pushLine({ kind: 'out', text: '201 created (blind signature received)', tone: 'ok' });
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'mintFinish(state, response)' });
			const credential = await mintFinish(state, response);
			await pushLine({
				kind: 'out',
				text: 'blind signature unblinded and verified locally',
				tone: 'ok'
			});

			const entry: WalletEntry = {
				id: ++entryCounter,
				credential,
				hpk: (credential as { hpk: string }).hpk,
				merchantId: keyset.merchant_id,
				issuedAt,
				used: false
			};
			wallet = [...wallet, entry];
			activeEntry = entry.id;
			await pushLine({ kind: 'stamp', text: 'credential stored', tone: 'ok' });
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

			await pushLine({ kind: 'cmd', text: 'publishReview(credential, body)' });
			const payload: ReviewPayload = await publishReview(entry.credential, body);
			await pushLine({
				kind: 'out',
				text: 'BBS+ presentation proof + ed25519 sig generated',
				tone: 'ok'
			});
			await pushLine({ kind: 'hr' });

			await pushLine({ kind: 'cmd', text: 'POST /api/v1/reviews' });
			await postReview(payload);
			await pushLine({
				kind: 'out',
				text: '201 created (server verified and registered)',
				tone: 'ok'
			});
			entry.used = true;
			reviewText = '';
			lastPublishedHpk = entry.hpk;
			await pushLine({ kind: 'stamp', text: 'review published', tone: 'ok' });
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
</script>

<main>
	<section class="left">
		<p class="kicker">reviewer / alice</p>
		<h1 class="title">wallet<span class="accent">.</span></h1>
		<p class="lede">
			mint a credential against an issuer's keyset, then publish a review. all secret material
			(hsk, blind factor, BBS+ signature) stays in this browser.
		</p>

		<div class="divider"></div>

		<div class="section-label">available issuers</div>
		{#each keysets as ks (ks.keyset_id)}
			<div class="merchant-card">
				<div class="row-mc">
					<div>
						<h2>{ks.merchant_id}</h2>
						<div class="keyset">kid_{ks.keyset_id}</div>
					</div>
					<button class="btn primary" disabled={busy} onclick={() => mint(ks)}>
						mint credential
					</button>
				</div>
			</div>
		{/each}

		<div class="section-label">wallet</div>
		{#if wallet.length === 0}
			<p class="kicker">no credentials yet.</p>
		{:else}
			{#each wallet as entry (entry.id)}
				<button
					class="credential"
					class:active={activeEntry === entry.id}
					class:used={entry.used}
					onclick={() => (activeEntry = entry.id)}
				>
					<div>
						<div class="kicker">{entry.merchantId}</div>
						<div class="hpk">hpk {shorten(entry.hpk)}</div>
					</div>
					<span class="badge">{entry.used ? 'spent' : 'fresh'}</span>
				</button>
			{/each}

			{#if findEntry(activeEntry) && !findEntry(activeEntry)!.used}
				<div class="composer">
					<div class="section-label">write review for {findEntry(activeEntry)!.merchantId}</div>
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
				</div>
			{/if}

			{#if lastPublishedHpk}
				<a href="/bob/" class="published-link">
					<span class="kicker">just published</span>
					<span>→ see it on bob</span>
				</a>
			{/if}
		{/if}
	</section>

	<Terminal title="WALLET LOG" subtitle={termSubtitle} lines={termLines} />
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
	.merchant-card {
		border: 1px solid var(--border);
		padding: 18px 22px;
		margin-bottom: 14px;
		background: var(--bg-soft);
	}
	.merchant-card .row-mc {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 18px;
	}
	.merchant-card h2 {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.merchant-card .keyset {
		color: var(--fg-dim);
		font-size: 11px;
		margin-top: 4px;
	}

	.credential {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 14px 18px;
		border: 1px solid var(--border);
		background: transparent;
		color: inherit;
		text-align: left;
		font: inherit;
		width: 100%;
		margin-bottom: 8px;
		cursor: pointer;
	}
	.credential.active {
		border-color: var(--accent);
	}
	.credential.used {
		opacity: 0.45;
	}
	.hpk {
		color: var(--fg-muted);
		font-size: 12px;
		margin-top: 4px;
	}
	.badge {
		font-size: 10px;
		letter-spacing: 0.15em;
		color: var(--accent);
		border: 1px solid var(--accent);
		padding: 3px 8px;
		text-transform: uppercase;
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
	.composer label {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: 12px;
		color: var(--fg-muted);
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
</style>
