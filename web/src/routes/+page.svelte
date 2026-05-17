<script lang="ts">
	import Term from '$lib/components/Term.svelte';

	const personas = [
		{
			slug: 'cafe',
			role: 'issuer',
			roleJa: '店',
			one: '会計と引き換えに receipt を客に発行する。'
		},
		{
			slug: 'alice',
			role: 'reviewer',
			roleJa: 'レビュアー',
			one: 'receipt を wallet に受け取り、レビューを公開する。'
		},
		{
			slug: 'bob',
			role: 'reader',
			roleJa: '読者',
			one: '公開レビューが本物かをブラウザだけで検証する。'
		},
		{
			slug: 'eve',
			role: 'attacker',
			roleJa: '攻撃者',
			one: '不正レビューを試して、仕組みが防げることを確認する。'
		}
	];
</script>

<main>
	<section class="hero">
		<p class="kicker">demo / chaumstar</p>
		<h1 class="title">verifiable anonymous reviews<span class="accent">.</span></h1>
		<p class="lede">
			匿名のまま、それでも<strong>本物の購入客が書いた</strong>と証明できるレビューのデモ。誰が書いたかは店にも読者にも分からないけれど、書いた人がそのお店の客であることだけは
			<Term
				tip="店から客に渡される「会計の証拠」。秘密のままレビューに紐づけられる。技術的には BBS+ blind signature による匿名 credential。"
			>credential</Term>
			で証明される。
		</p>
	</section>

	<div class="section-label">demo が示すこと</div>
	<ul class="claims">
		<li>
			<span class="num">01</span>
			<div>
				<div class="claim-head">レビュアーは本物の購入客</div>
				<div class="claim-sub">店から credential を渡された人だけがレビューを書ける。</div>
			</div>
		</li>
		<li>
			<span class="num">02</span>
			<div>
				<div class="claim-head">誰が書いたかは分からない</div>
				<div class="claim-sub">店も読者も、レビュアーの正体を辿れない。</div>
			</div>
		</li>
		<li>
			<span class="num">03</span>
			<div>
				<div class="claim-head">1つの credential で1レビュー</div>
				<div class="claim-sub">同じ会計を使って2回レビューを書くことはできない。</div>
			</div>
		</li>
		<li>
			<span class="num">04</span>
			<div>
				<div class="claim-head">改ざんを読者が検出</div>
				<div class="claim-sub">登録後に本文を書き換えたり、レビューを密かに消したりすると検出される。</div>
			</div>
		</li>
	</ul>

	<div class="divider"></div>

	<div class="section-label">登場人物</div>
	<div class="personas">
		{#each personas as p (p.slug)}
			<a href={`/${p.slug}/`} class="persona" class:attack={p.slug === 'eve'}>
				<div class="kicker">{p.role}</div>
				<div class="big">{p.slug} <span class="ja">({p.roleJa})</span></div>
				<p class="ptext">{p.one}</p>
			</a>
		{/each}
	</div>

	<div class="section-label">推奨フロー (mainline)</div>
	<div class="track">
		<a class="node" href="/cafe/">
			<span class="n-step">01</span>
			<span class="n-name">cafe</span>
			<span class="n-role">店</span>
			<span class="n-act">receipt 発行</span>
		</a>
		<span class="arrow" aria-hidden="true">→</span>
		<a class="node" href="/alice/">
			<span class="n-step">02</span>
			<span class="n-name">alice</span>
			<span class="n-role">レビュアー</span>
			<span class="n-act">claim → publish</span>
		</a>
		<span class="arrow" aria-hidden="true">→</span>
		<a class="node" href="/bob/">
			<span class="n-step">03</span>
			<span class="n-name">bob</span>
			<span class="n-role">読者</span>
			<span class="n-act">独立に検証</span>
		</a>
	</div>

	<div class="section-label">攻撃シナリオ (off-path)</div>
	<a class="node attack standalone" href="/eve/">
		<div class="atk-head">
			<span class="n-step">opt</span>
			<span class="n-name">eve</span>
			<span class="n-role">攻撃者</span>
		</div>
		<span class="n-act">
			3種の攻撃 (二重投稿 / 本文改ざん / 証明偽造) を実際にサーバに送り、すべて拒否されることを確認する。
		</span>
	</a>

	<div class="actions">
		<a class="btn primary" href="/cafe/">→ cafe からはじめる</a>
		<a class="btn" href="/bob/">→ bob でレビューを見る</a>
	</div>

	<div class="divider"></div>
	<p class="kicker">本デモは状態を永続化しません。ページ再読込でリセットされます。</p>
</main>

<style>
	main {
		max-width: 880px;
		margin: 0 auto;
		padding: 48px 56px 96px;
	}
	.hero {
		margin-bottom: 40px;
	}
	.lede strong {
		color: var(--fg);
	}
	.claims {
		list-style: none;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 14px;
	}
	.claims li {
		display: flex;
		gap: 14px;
		align-items: baseline;
		border: 1px solid var(--border);
		padding: 16px 20px;
		background: var(--bg-soft);
		border-radius: 3px;
	}
	.num {
		color: var(--accent);
		font-size: 12px;
		letter-spacing: 0.15em;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.claim-head {
		font-size: 15px;
		color: var(--fg);
		font-weight: 600;
	}
	.claim-sub {
		font-size: 13px;
		color: var(--fg-muted);
		margin-top: 4px;
		line-height: 1.6;
	}

	.personas {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 14px;
		margin-bottom: 8px;
	}
	.persona {
		display: block;
		border: 1px solid var(--border);
		padding: 18px 22px;
		background: var(--bg-soft);
		text-decoration: none;
		color: inherit;
		transition: border-color 0.12s;
		border-radius: 3px;
	}
	.persona:hover {
		border-color: var(--accent);
	}
	.persona.attack:hover {
		border-color: var(--err);
	}
	.big {
		font-size: 22px;
		font-weight: 600;
		letter-spacing: -0.01em;
		margin: 4px 0 4px;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.big .ja {
		font-size: 13px;
		font-weight: 400;
		color: var(--fg-muted);
		margin-left: 4px;
		font-family: inherit;
	}
	.ptext {
		color: var(--fg-muted);
		font-size: 13px;
		line-height: 1.7;
		margin-top: 6px;
	}

	.track {
		display: flex;
		align-items: stretch;
		gap: 0;
		margin-top: 4px;
	}
	.node {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 14px 18px;
		border: 1px solid var(--border);
		background: var(--bg-soft);
		text-decoration: none;
		color: inherit;
		min-width: 0;
		transition: border-color 0.12s;
		border-radius: 3px;
	}
	.track .node {
		flex: 1;
	}
	.node:hover {
		border-color: var(--accent);
	}
	.node.attack {
		border-style: dashed;
		border-color: var(--accent-soft);
	}
	.node.attack:hover {
		border-color: var(--err);
		border-style: dashed;
	}
	.node.standalone {
		display: block;
		padding: 16px 20px;
		margin-top: 6px;
	}
	.n-step {
		color: var(--accent);
		font-size: 10px;
		letter-spacing: 0.18em;
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.n-name {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.01em;
		color: var(--fg);
		font-family: 'JetBrains Mono', ui-monospace, monospace;
	}
	.n-role {
		font-size: 11px;
		color: var(--fg-muted);
	}
	.n-act {
		margin-top: 4px;
		font-size: 12px;
		color: var(--fg-muted);
		line-height: 1.6;
	}
	.arrow {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0 14px;
		color: var(--accent);
		font-size: 22px;
	}
	.atk-head {
		display: flex;
		align-items: baseline;
		gap: 12px;
		margin-bottom: 6px;
	}

	.actions {
		margin-top: 28px;
		display: flex;
		gap: 12px;
		flex-wrap: wrap;
	}

	@media (max-width: 720px) {
		main {
			padding: 28px 16px 64px;
		}
		.hero {
			margin-bottom: 28px;
		}
		.claims,
		.personas {
			grid-template-columns: 1fr;
		}
		.track {
			flex-direction: column;
		}
		.track .node {
			flex: none;
		}
		.arrow {
			padding: 4px 0;
			transform: rotate(90deg);
		}
		.big {
			font-size: 18px;
		}
		.n-name {
			font-size: 16px;
		}
		.node.standalone {
			padding: 14px 16px;
		}
	}
</style>
