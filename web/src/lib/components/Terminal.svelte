<script lang="ts">
	export type Line =
		| { kind: 'cmd'; text: string; tip?: string }
		| { kind: 'out'; text: string; tone?: 'ok' | 'warn' | 'err' | 'plain'; tip?: string }
		| { kind: 'hr' }
		| { kind: 'stamp'; text: string; tone?: 'ok' | 'err'; tip?: string };

	let { title = 'BBS+ PROOF LOG', subtitle = '', lines = [] }: { title?: string; subtitle?: string; lines: Line[] } = $props();
</script>

<aside class="term">
	<div class="term-header">
		<span class="title">{title}</span>
		<span class="kid">{subtitle}</span>
	</div>
	<div class="term-body">
		{#each lines as line, i (i)}
			{#if line.kind === 'cmd'}
				<div class="line" class:tipped={line.tip}>
					<span class="prompt">$</span> {line.text}
					{#if line.tip}<span class="tip-pop">{line.tip}</span>{/if}
				</div>
			{:else if line.kind === 'out'}
				<div class="out {line.tone ?? 'plain'}" class:tipped={line.tip}>
					{line.text}
					{#if line.tip}<span class="tip-pop">{line.tip}</span>{/if}
				</div>
			{:else if line.kind === 'hr'}
				<hr />
			{:else if line.kind === 'stamp'}
				<div class="stamp" class:err={line.tone === 'err'} class:tipped={line.tip}>
					{line.text}
					{#if line.tip}<span class="tip-pop">{line.tip}</span>{/if}
				</div>
			{/if}
		{/each}
	</div>
</aside>

<style>
	.term {
		background: var(--bg-deepest);
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		position: sticky;
		top: 0;
		height: 100vh;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	@media (max-width: 720px) {
		.term {
			position: static;
			height: auto;
			max-height: 60vh;
			border-left: none;
			border-top: 1px solid var(--border);
		}
		.term-header {
			padding: 10px 14px;
		}
		.term-body {
			padding: 14px 14px;
			font-size: 12px;
		}
	}
	.term-header {
		padding: 12px 18px;
		border-bottom: 1px solid var(--border);
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: var(--bg);
	}
	.term-header .title {
		font-size: 11px;
		color: var(--fg-muted);
		letter-spacing: 0.2em;
		text-transform: uppercase;
	}
	.term-header .kid {
		font-size: 11px;
		color: var(--fg-dim);
	}
	.term-body {
		padding: 22px 18px;
		flex: 1;
		overflow-y: auto;
		font-size: 13px;
		line-height: 1.7;
	}
	.term-body > div,
	.term-body > hr {
		animation: appear 0.18s ease-out;
	}
	@keyframes appear {
		from {
			opacity: 0;
			transform: translateY(-2px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	.term-body .line {
		color: var(--fg-muted);
		position: relative;
	}
	.term-body .prompt {
		color: var(--accent);
		user-select: none;
		margin-right: 6px;
	}
	.term-body .out {
		color: var(--fg);
		padding-left: 14px;
		position: relative;
	}
	.term-body .out.ok::before {
		content: '✓ ';
		color: var(--ok);
	}
	.term-body .out.warn::before {
		content: '→ ';
		color: var(--warn);
	}
	.term-body .out.err::before {
		content: '✗ ';
		color: var(--err);
	}
	.term-body hr {
		border: none;
		border-top: 1px dashed var(--border-strong);
		margin: 10px 0;
	}
	.term-body .stamp {
		margin-top: 24px;
		padding: 10px 14px;
		border: 1px solid var(--accent);
		color: var(--accent);
		font-weight: 600;
		letter-spacing: 0.25em;
		font-size: 12px;
		display: inline-block;
		text-transform: uppercase;
		position: relative;
	}
	.term-body .stamp::before {
		content: '▌';
	}
	.term-body .stamp::after {
		content: '▐';
	}
	.term-body .stamp.err {
		border-color: var(--err);
		color: var(--err);
	}

	.term-body .tipped {
		cursor: help;
		padding-top: 2px;
		padding-bottom: 2px;
		border-radius: 2px;
	}
	.term-body .tipped:hover {
		background: rgba(232, 178, 92, 0.07);
	}
	.term-body .tip-pop {
		display: none;
		margin-top: 6px;
		padding: 8px 12px;
		background: var(--bg);
		color: var(--fg);
		border-left: 2px solid var(--accent);
		font-size: 11.5px;
		line-height: 1.65;
		font-family:
			-apple-system,
			BlinkMacSystemFont,
			'Inter',
			'Hiragino Sans',
			'Noto Sans JP',
			sans-serif;
		letter-spacing: 0;
		text-transform: none;
		font-weight: 400;
		text-align: left;
		white-space: normal;
	}
	.term-body .tipped:hover .tip-pop,
	.term-body .tipped:focus-within .tip-pop {
		display: block;
	}
</style>
