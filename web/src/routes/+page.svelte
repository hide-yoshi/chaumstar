<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		goto('/bob/', { replaceState: true });
	});
</script>

<main class="landing">
	<p class="kicker">redirecting…</p>
</main>

<style>
	.landing {
		padding: 56px;
	}
</style>
