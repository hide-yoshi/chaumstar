<script lang="ts">
	import { onMount } from 'svelte';
	import { listKeysets } from '$lib/api';
	import type { PublicKeyset } from '$lib/types';
	import Terminal, { type Line } from '$lib/components/Terminal.svelte';

	let keysets = $state<PublicKeyset[]>([]);
	let termSubtitle = $state<string>('');
	let termLines = $state<Line[]>([
		{ kind: 'cmd', text: 'GET /api/v1/keysets' },
		{ kind: 'out', text: 'fetching…', tone: 'warn' }
	]);

	onMount(async () => {
		keysets = await listKeysets();
		termSubtitle = `${keysets.length} keyset${keysets.length === 1 ? '' : 's'}`;
		termLines = [
			{ kind: 'cmd', text: 'GET /api/v1/keysets' },
			{ kind: 'out', text: `200 ok · ${keysets.length} record(s)`, tone: 'ok' },
			{ kind: 'hr' },
			...keysets.flatMap((ks) => [
				{ kind: 'cmd' as const, text: `kid_${ks.keyset_id}` },
				{
					kind: 'out' as const,
					text: `${ks.issuer_id} / ${ks.merchant_id}`,
					tone: 'ok' as const
				},
				{
					kind: 'out' as const,
					text: `PK_m = ${shorten(ks.public_key_bytes)}`,
					tone: 'warn' as const
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
		<p class="kicker">issuer / cafe</p>
		<h1 class="title">mint keysets<span class="accent">.</span></h1>
		<p class="lede">
			the server holds one BBS+ secret key per (issuer, merchant). reviewers fetch the public
			keyset, then commit to <code>hpk</code> and request a blind signature.
		</p>

		<div class="divider"></div>

		{#if keysets.length === 0}
			<p class="kicker">loading…</p>
		{:else}
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

			<div class="note">
				<div class="kicker">how mint works</div>
				<ol>
					<li>reviewer commits to <code>hpk</code> with a Pedersen-style commitment</li>
					<li>POST <code>/api/v1/mints</code> ← <code>{`{commitment, merchant_id, issued_at}`}</code></li>
					<li>issuer runs <code>BlindSignature::blind_sign(SK_m, commit, [mid, ts])</code></li>
					<li>reviewer unblinds locally; secret material never leaves the wallet</li>
				</ol>
			</div>
		{/if}
	</section>

	<Terminal title="ISSUER STATE" subtitle={termSubtitle} lines={termLines} />
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
	.note {
		margin-top: 36px;
		border-left: 2px solid var(--accent);
		padding: 14px 22px;
		background: var(--bg-soft);
	}
	.note ol {
		margin-top: 10px;
		padding-left: 20px;
		font-size: 13px;
		color: var(--fg-muted);
	}
	.note li {
		margin-bottom: 8px;
	}
	code {
		color: var(--accent);
		background: var(--bg);
		padding: 1px 4px;
		font-size: 0.95em;
	}
</style>
