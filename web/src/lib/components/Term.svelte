<script lang="ts">
	import type { Snippet } from 'svelte';

	let { tip, children }: { tip: string; children: Snippet } = $props();
	let open = $state(false);
	let wrap: HTMLElement | undefined = $state();

	function toggle(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		open = !open;
	}

	function close() {
		open = false;
	}

	$effect(() => {
		if (!open) return;
		const onDoc = (e: MouseEvent) => {
			if (wrap && !wrap.contains(e.target as Node)) close();
		};
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') close();
		};
		const t = window.setTimeout(() => {
			document.addEventListener('click', onDoc);
			document.addEventListener('keydown', onKey);
		}, 0);
		return () => {
			window.clearTimeout(t);
			document.removeEventListener('click', onDoc);
			document.removeEventListener('keydown', onKey);
		};
	});
</script>

<span class="wrap" bind:this={wrap}>
	<button
		type="button"
		class="term"
		class:open
		onclick={toggle}
		aria-expanded={open}
		aria-haspopup="true"
	>
		{@render children()}
	</button>
	<span class="tip" class:open role="tooltip">{tip}</span>
</span>

<style>
	.wrap {
		position: relative;
		display: inline-block;
		line-height: 1.2;
	}
	.term {
		font: inherit;
		padding: 0;
		background: none;
		border: none;
		border-bottom: 1px dotted var(--accent);
		color: var(--accent);
		cursor: help;
	}
	.term:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.tip {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
		width: max-content;
		max-width: 280px;
		background: var(--bg-soft);
		color: var(--fg);
		border: 1px solid var(--border-strong);
		border-left: 2px solid var(--accent);
		padding: 10px 14px;
		font-size: 12px;
		line-height: 1.65;
		opacity: 0;
		pointer-events: none;
		transition:
			opacity 0.15s,
			transform 0.15s;
		z-index: 100;
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.4);
		font-family:
			-apple-system,
			BlinkMacSystemFont,
			'Inter',
			'Hiragino Sans',
			'Noto Sans JP',
			sans-serif;
		text-transform: none;
		letter-spacing: 0;
		font-weight: 400;
		text-align: left;
		white-space: normal;
		border-radius: 2px;
	}
	.wrap:hover .tip,
	.tip.open {
		opacity: 1;
		pointer-events: auto;
	}
	.tip::after {
		content: '';
		position: absolute;
		top: 100%;
		left: 50%;
		transform: translateX(-50%);
		border: 6px solid transparent;
		border-top-color: var(--bg-soft);
	}

	@media (max-width: 640px) {
		.tip {
			max-width: 220px;
		}
	}
</style>
