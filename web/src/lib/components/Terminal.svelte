<script lang="ts">
	export type Line =
		| { kind: 'cmd'; text: string }
		| { kind: 'out'; text: string; tone?: 'ok' | 'warn' | 'err' | 'plain' }
		| { kind: 'hr' }
		| { kind: 'stamp'; text: string; tone?: 'ok' | 'err' };

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
				<div class="line"><span class="prompt">$</span> {line.text}</div>
			{:else if line.kind === 'out'}
				<div class="out {line.tone ?? 'plain'}">{line.text}</div>
			{:else if line.kind === 'hr'}
				<hr />
			{:else if line.kind === 'stamp'}
				<div class="stamp" class:err={line.tone === 'err'}>{line.text}</div>
			{/if}
		{/each}
	</div>
</aside>

<style>
	.term {
		background: #080808;
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		position: sticky;
		top: 0;
		height: 100vh;
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
	}
	.term-body .prompt {
		color: var(--accent);
		user-select: none;
		margin-right: 6px;
	}
	.term-body .out {
		color: var(--fg);
		padding-left: 14px;
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
		border-top: 1px dashed #333;
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
</style>
