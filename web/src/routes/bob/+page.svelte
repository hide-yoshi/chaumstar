<script lang="ts">
	import { onMount } from 'svelte';
	import { listKeysets, listReviews } from '$lib/api';
	import { ensureWasm, verifyProof } from '$lib/wasm';
	import type { PublicKeyset, ReviewPayload } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';
	import { LINE_DELAY, delay } from '$lib/anim';

	type Status = 'pending' | 'verifying' | 'valid' | 'invalid';
	interface Row {
		payload: ReviewPayload;
		status: Status;
		error?: string;
	}

	let keysets = $state<PublicKeyset[]>([]);
	let rows = $state<Row[]>([]);
	let selected = $state<number | null>(null);
	let termLines = $state<Line[]>([]);
	let termSubtitle = $state<string>('');
	let refreshing = $state<boolean>(false);
	let renderToken = 0;

	onMount(async () => {
		await ensureWasm();
		await refresh();
	});

	async function refresh(): Promise<void> {
		refreshing = true;
		try {
			keysets = await listKeysets();
			const reviews = await listReviews();
			rows = reviews.map((p) => ({ payload: p, status: 'pending' as Status }));
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
		const script: Line[] = [
			{ kind: 'cmd', text: `resolve_keyset(${row.payload.credential_proof.keyset_id})` },
			{ kind: 'out', text: 'PK_m loaded (BLS12-381 G2, 96 B)', tone: ok ? 'ok' : 'err' },
			{ kind: 'hr' },
			{ kind: 'cmd', text: 'canonicalize(review_body, hpk, kid)' },
			{ kind: 'out', text: 'M_jcs ready', tone: 'warn' },
			{ kind: 'hr' },
			{ kind: 'cmd', text: 'ed25519.verify(hpk, M_jcs, σ_ed)' },
			{
				kind: 'out',
				text: ok ? 'holder signature valid' : 'verification step failed',
				tone: ok ? 'ok' : 'err'
			},
			{ kind: 'hr' },
			{ kind: 'cmd', text: 'presentation_header = sha256(M_jcs)' },
			{ kind: 'out', text: 'bound to review body', tone: 'warn' },
			{ kind: 'hr' },
			{ kind: 'cmd', text: 'bbs.proof.verify(π, PK_m, [hpk, mid, ts])' },
			{
				kind: 'out',
				text: ok ? 'presentation proof valid' : 'presentation proof FAILED',
				tone: ok ? 'ok' : 'err'
			},
			{ kind: 'stamp', text: ok ? 'authentic' : 'rejected', tone: ok ? 'ok' : 'err' }
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

	function pickRow(i: number) {
		selected = i;
		renderTerminal(rows[i]);
	}

	function shorten(hex: string, head = 4, tail = 2): string {
		if (hex.length <= head + tail + 3) return hex;
		return `${hex.slice(0, head)}...${hex.slice(-tail)}`;
	}

	function stars(rating: number): string {
		return '★'.repeat(rating).padEnd(5, '☆');
	}
</script>

<main>
	<section class="left">
		<p class="kicker">reader / bob</p>
		<h1 class="title">verified purchaser reviews<span class="accent">.</span></h1>
		<p class="lede">
			each review carries a BBS+ presentation proof. verification runs locally in your browser
			(WASM); the registry's claim is not trusted.
		</p>

		<div class="actions">
			<button class="btn" disabled={refreshing} onclick={refresh}>
				{refreshing ? 'refreshing…' : '↻ refresh'}
			</button>
		</div>

		<div class="divider"></div>

		{#if keysets.length === 0}
			<p class="kicker">loading keysets…</p>
		{:else}
			{#each keysets as ks (ks.keyset_id)}
				{@const ksRows = rows.filter((r) => r.payload.credential_proof.keyset_id === ks.keyset_id)}
				<div class="merchant-card">
					<div class="row-mc">
						<h2>{ks.merchant_id}</h2>
						<span class="count">{ksRows.length} verified</span>
					</div>
					<div class="keyset">
						issuer: {ks.issuer_id} · kid_{ks.keyset_id} · PK_m (96 B G2)
					</div>
				</div>
			{/each}
		{/if}

		{#if rows.length === 0 && keysets.length > 0}
			<p class="kicker">no reviews yet. switch to alice and publish one.</p>
		{/if}

		<div class="reviews">
			{#each rows as row, i (row.payload.credential_proof.hpk)}
				<button
					class="review"
					class:active={selected === i}
					class:invalid={row.status === 'invalid'}
					onclick={() => pickRow(i)}
				>
					<span class="stars">{stars(row.payload.review_body.rating)}</span>
					<div class="body">
						<p class="text">{row.payload.review_body.text}</p>
						<div class="meta">
							<span class="hpk">{shorten(row.payload.credential_proof.hpk)}</span>
							<span> · {row.payload.review_body.timestamp}</span>
						</div>
					</div>
					<span class="badge" class:badge-err={row.status === 'invalid'}>
						{row.status === 'valid' ? 'verified' : row.status}
					</span>
				</button>
			{/each}
		</div>
	</section>

	<Terminal title="BBS+ PROOF LOG" subtitle={termSubtitle} lines={termLines} />
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
		margin-bottom: 28px;
		background: var(--bg-soft);
	}
	.merchant-card .row-mc {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}
	.merchant-card h2 {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.merchant-card .count {
		color: var(--fg-muted);
		font-size: 12px;
	}
	.merchant-card .keyset {
		color: var(--fg-dim);
		font-size: 11px;
		margin-top: 8px;
	}

	.reviews {
		display: flex;
		flex-direction: column;
	}
	.review {
		text-align: left;
		display: grid;
		grid-template-columns: 50px 1fr auto;
		gap: 18px;
		align-items: start;
		padding: 22px 4px;
		border-top: 1px solid var(--border);
		cursor: pointer;
		transition: background 0.12s;
		font: inherit;
		color: inherit;
		background: transparent;
	}
	.review:last-child {
		border-bottom: 1px solid var(--border);
	}
	.review:hover {
		background: var(--bg-soft);
	}
	.review.active {
		background: var(--bg-soft);
		border-left: 2px solid var(--accent);
		padding-left: 14px;
	}
	.review.invalid {
		border-left: 2px solid var(--err);
	}
	.stars {
		font-weight: 500;
		color: var(--accent);
		font-size: 14px;
		letter-spacing: 0.05em;
	}
	.text {
		font-size: 15px;
		color: var(--fg);
		line-height: 1.5;
		margin-bottom: 6px;
	}
	.meta {
		font-size: 11px;
		color: var(--fg-dim);
	}
	.meta .hpk {
		color: var(--fg-muted);
	}
	.badge {
		font-size: 10px;
		letter-spacing: 0.15em;
		color: var(--accent);
		border: 1px solid var(--accent);
		padding: 3px 8px;
		white-space: nowrap;
		text-transform: uppercase;
		align-self: flex-start;
	}
	.badge.badge-err {
		color: var(--err);
		border-color: var(--err);
	}
	.actions {
		margin-top: 18px;
	}
</style>
