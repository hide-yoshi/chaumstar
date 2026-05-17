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
		{ slug: 'cafe', label: 'cafe', role: '店' },
		{ slug: 'alice', label: 'alice', role: 'レビュアー' },
		{ slug: 'bob', label: 'bob', role: '読者' },
		{ slug: 'eve', label: 'eve', role: '攻撃者' }
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
			<a href={`/${p.slug}/`} class:active={isActive(p.slug)}>
				<span class="nm">{p.label}</span>
				<span class="role">{p.role}</span>
			</a>
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
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1px;
		line-height: 1.2;
	}
	nav a .role {
		font-size: 10px;
		color: var(--fg-dim);
		letter-spacing: 0.05em;
	}
	nav a.active .role {
		color: var(--fg-muted);
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
		padding: 4px 28px;
		font-size: 9.5px;
		letter-spacing: 0.04em;
		color: var(--fg-dim);
		opacity: 0.55;
		display: flex;
		gap: 14px;
		flex-wrap: wrap;
	}

	@media (max-width: 720px) {
		header {
			padding: 10px 14px;
		}
		.logo {
			font-size: 13px;
		}
		nav a {
			padding: 4px 8px;
			font-size: 11px;
		}
		nav a .role {
			font-size: 9px;
		}
		.status-bar {
			padding: 3px 14px;
			gap: 10px;
			font-size: 8.5px;
		}
	}
</style>
