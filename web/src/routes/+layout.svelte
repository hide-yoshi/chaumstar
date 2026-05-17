<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { getHealth, listKeysets } from '$lib/api';

	let { children } = $props();

	let health = $state<string>('?');
	let version = $state<string>('chaumstar/0.1');
	let keysetCount = $state<number>(0);
	let lastError = $state<string>('');

	const personas = [
		{ slug: 'cafe', label: 'cafe' },
		{ slug: 'alice', label: 'alice' },
		{ slug: 'bob', label: 'bob' },
		{ slug: 'eve', label: 'eve' }
	];

	function isActive(slug: string): boolean {
		return page.url.pathname.startsWith(`/${slug}`);
	}

	onMount(async () => {
		try {
			const h = await getHealth();
			health = h.status;
			version = h.version;
			const ks = await listKeysets();
			keysetCount = ks.length;
		} catch (e) {
			lastError = String(e);
			health = 'err';
		}
	});
</script>

<header>
	<a class="logo" href="/">chaumstar<span class="dot">.</span></a>
	<nav>
		{#each personas as p (p.slug)}
			<a href={`/${p.slug}/`} class:active={isActive(p.slug)}>{p.label}</a>
		{/each}
	</nav>
</header>

<div class="status-bar">
	<span>{version}</span>
	<span>GET /api/v1/health <span class={health === 'ok' ? 'ok' : 'err'}>{health}</span></span>
	<span>{keysetCount} keyset{keysetCount === 1 ? '' : 's'}</span>
	{#if lastError}<span class="err">{lastError}</span>{/if}
</div>

{@render children()}

<style>
	header {
		border-bottom: 1px solid var(--border);
		padding: 14px 28px;
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.logo {
		font-weight: 600;
		letter-spacing: -0.02em;
	}
	.logo .dot {
		color: var(--accent);
	}
	nav {
		display: flex;
		gap: 0;
		border: 1px solid var(--border);
	}
	nav a {
		color: var(--fg-muted);
		border-right: 1px solid var(--border);
		padding: 6px 14px;
		text-transform: lowercase;
		font-size: 12px;
		text-decoration: none;
	}
	nav a:last-child {
		border-right: none;
	}
	nav a.active {
		color: var(--fg);
		background: var(--bg-soft);
		box-shadow: inset 0 -2px var(--accent);
	}
	.status-bar {
		border-bottom: 1px solid var(--border);
		padding: 6px 28px;
		font-size: 11px;
		color: var(--fg-dim);
		display: flex;
		gap: 18px;
		flex-wrap: wrap;
	}
</style>
