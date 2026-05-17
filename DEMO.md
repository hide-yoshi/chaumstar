# chaumstar Demo — v0.3 (draft)

> Single-page web demo. "Hacker News に投稿された1分動画で何が起きてるか伝わる" を目標にする。
>
> - v0.2: **selective disclosure** (`purchase_tier` / `product_category`)
> - v0.3: **Registry transparency log** (Merkle log + 署名付き STH + inclusion proof) で Registry 側の削除 / 改ざんを Reader が検出可能に

---

## 1. ゴール

このデモが**技術的に証明**すること:

- Issuer は Reviewer の身元を知らない (BBS+ blind issuance)
- Reader は Reviewer の身元を知らない (BBS+ ZK proof; signature 非開示)
- Issuer が mint log を持っていても publish と紐づけられない (proof randomization)
- 同じ credential で2回レビューはできない (linkability via nullifier `hpk`)
- レビュー本文の改ざんを Reader が検出できる (Ed25519 sig + presentation_header binding)
- 偽の Issuer 鍵では credential が作れない (BBS+ proof verify against PK_m)
- **Reviewer は属性を開示するか / 隠すか選べるが、値を改竄できない** (v0.2 selective disclosure)
- **Registry の削除 / 改ざんを Reader が独立に検出できる** (v0.3 Merkle transparency log + 署名付き STH)

このデモが**物語として伝える**こと:

- レビューが Issuer の祝福を受けた誰かによって書かれたことを、Reader が独立に暗号的に検証できる
- Issuer も Reader も、レビューを特定の購入イベント・購入者に紐づけられない

---

## 2. Scope

### In-scope

- Mint flow (QRコード経由でcredential取得)
- Publish flow (wallet からレビュー作成・公開)
- Verify flow (公開レビューの暗号検証 + バッジ表示)
- 攻撃デモ 3種 (double-spend / tamper / forge)
- ステップ毎の暗号操作の可視化 (B', C', DLEQ verify 等が画面に出る)

### Out-of-scope

- ❌ Moderation UI (実運用では必須だが、デモでは混乱を避けるため省略)
- ❌ 実際の payment processor 連携 (Stripe webhook 等)
- ❌ 永続化 (再起動でリセット = デモのリプレイ性向上)
- ❌ Multi-issuer / multi-merchant 切り替え
- ❌ Reviewer 識別子の永続化 (毎回 fresh wallet)
- ❌ 商用UI のクオリティ (機能性優先)
- ❌ Mobile native app

---

## 3. シナリオ

舞台: **Bean & Beam Coffee** (架空のサードウェーブ・コーヒー店)

```
[店内のテーブル] 「ご来店ありがとうございます。 
                   QR をスキャンしてレビューを書いてください」
                   [大きなQRコード]
```

1. Alice が会計後、QR をスキャン → 自分の wallet に credential が入る
2. Alice が帰宅後、wallet からレビューを書いて公開
3. これから行こうとしている Bob が、公開レビュー一覧を閲覧
4. 各レビューに「Verified Purchaser ✓」 — 暗号バッジが付いている
5. Bob が一つのレビューをクリック → 検証ログが展開 → 全ステップ ✓

---

## 4. アクター (Persona switcher)

Single-page で persona ボタンを切り替えながら flow を進める。

| ペルソナ | 役割 |
|---|---|
| 🏪 **Bean & Beam Coffee** (Issuer) | Mint API + keyset 公開 |
| 👩 **Alice** (Reviewer) | Wallet 操作、レビュー公開 |
| 👨 **Bob** (Reader) | 公開レビュー閲覧・検証 |
| 🦹 **Eve** (Attacker) | 攻撃デモ用 persona |

---

## 5. 画面構成

```
┌──────────────────────────────────────────────────────┐
│  chaumstar demo                                      │
│  [🏪 Cafe] [👩 Alice] [👨 Bob] [🦹 Eve]              │
└──────────────────────────────────────────────────────┘
```

### 🏪 Cafe view

- Issuer 公開鍵表示 (keyset list, BLS12-381 G₂ point)
- **会計フォーム** (v0.2):
  - `¥ amount` 入力 → 自動で `purchase_tier` 算出 (¥0-999 = low / ¥1,000-4,999 = mid / ¥5,000+ = high)
  - `category` プルダウン (drinks / food / merch)
  - "💰 mint trigger" ボタン → mint context が生成され、Alice 側で credential 取得可能に
- Mint log: どんな request を受けたかを表示
  - **重要**: ここで `C_blind` (Pedersen commitment) しか見えないことを強調 →「Issuer は hpk を知らない」

### 👩 Alice view

- "📷 QR をスキャン" ボタン (実際にはモーダルでQRを表示してsimulate)
- Wallet UI: 保有 credential 一覧
  - 各 credential の `hpk`, `C`, `keyset_id`, `tier`, `category` を表示
- レビュー作成フォーム:
  - `rating` (1-5)
  - `text`
  - **Disclosure mask** checkboxes (v0.2):
    - `[ ] reveal purchase_tier`  (デフォルト OFF)
    - `[ ] reveal product_category`  (デフォルト OFF)
- "📝 レビューを公開" ボタン
- 操作ごとに**暗号ステップが右ペインに流れる** (Mint flow → Publish flow)

### 👨 Bob view

- 公開レビュー一覧 (cafeに対するレビュー)
  - "美味しかった ⭐⭐⭐⭐⭐ [✓ Verified Purchaser] [tier: mid] [drinks]"
  - 開示されてない属性は表示されない (reviewer の選択を尊重)
- **Filter UI** (v0.2):
  - `[ ] only show tier=high`
  - `[ ] only show category=drinks`
- 任意のレビューをクリック → 「🔍 検証ログ」がスライドオープン
  - ✓ Keyset resolved: kid_abc123 → PK_m (BLS12-381 G₂)
  - ✓ Disclosed indexes computed from payload: [0, 1, 2] (tier disclosed)
  - ✓ BBS+ blind proof verified: issuer signed (hpk, merchant_id, issued_at, tier=mid)
  - ✓ Presentation header binds proof to review body + disclosure
  - ✓ Ed25519 sig verified: review text untampered
  - ✓ Nullifier check: hpk unused in registry
  - **VALID**

### 🦹 Eve view

- 攻撃選択メニュー (3 ボタン)
  - "🔁 同じ credential で2回目レビューを試す" (Double-spend)
  - "✂️ Alice のレビュー本文を改ざんする" (Tamper)
  - "🎭 Issuer 鍵なしで credential を偽造する" (Forge)
- 各攻撃の試行結果 + 「なぜ失敗したか」の暗号的説明

---

## 6. Happy path (E2E)

```
[Cafe]   "💰 お客様が会計しました" 押下
         → 新規 QR (一回限り mint token) を画面に表示
         
[Alice]  "📷 QR をスキャン"
         右ペイン:
           1. (hsk, hpk) ← Ed25519 keygen          ✓
           2. m₁ = H_s(hpk), m₂ = H_s(mid),
              m₃ = H_s(issued_at)                  ✓
           3. ρ ← random Z_r                       ✓
           4. C_blind = [m₁]H₁ + [ρ]H₀             ✓
           5. pok_commit = PoK(m₁, ρ)              ✓
           6. → POST /api/issuer/mint
                (C_blind, pok, merchant_id, ...)   ✓
           7. ← blind_σ                            ✓
           8. σ = unblind(blind_σ, ρ)              ✓
           9. bbs_verify(σ, [m₁,m₂,m₃], PK_m)      ✓
         Wallet に credential 追加 (σ は wallet 内に秘匿)
         
[Alice]  レビュー: "ハンドドリップが秀逸 ⭐⭐⭐⭐⭐"
         "📝 レビューを公開" 押下
         右ペイン:
           1. M = JCS(review_body + hpk + kid)     ✓
           2. presentation_header = SHA256(M)      ✓
           3. π = bbs_create_proof(
                 σ, [m₁,m₂,m₃], all_revealed,
                 presentation_header, PK_m)        ✓
           4. σ_ed = Ed25519.sign(hsk, M)          ✓
           5. → POST /api/registry/publish         ✓
           6. registry.spent_set.add(hpk)          ✓
         
[Bob]    レビュー一覧を開く
         "ハンドドリップが秀逸 ⭐⭐⭐⭐⭐  [✓ Verified Purchaser]"
         クリック → 検証ログ展開 → 全項目 ✓
```

---

## 7. 攻撃デモ

### Attack 1: Double-spend (二重投稿)

```
[Alice]  レビュー1を公開 (上記 happy path)
[Eve]    (Alice の wallet を一時的に借りた想定で switch)
         同じ credential を選択 → 別レビュー文を入力 → 公開
         
         → POST /api/registry/publish
         → registry: hpk already in spent_set
         → REJECTED
         
表示: 🚫 Double-review detected.
      This credential (hpk = ab12...) has already been used.
      First use: 2 minutes ago
```

### Attack 2: Tampering (本文改ざん)

```
[Eve]    Alice の公開レビューを取得 (ネットワーク経路で intercept した想定)
         本文を "❌ 最悪だった ⭐" に書き換え
         元の payload に新しい本文を入れて再投稿
         
         → POST /api/registry/publish
         → recompute M' from tampered body
         → Ed25519.verify(hpk, M', σ_ed) → FAIL
         → (additionally) bbs proof's presentation_header = SHA256(M)
           was bound to original M; recomputed SHA256(M') ≠ original
           → bbs_verify_proof → FAIL
         → REJECTED
         
表示: 🚫 Tamper detected.
      Both Ed25519 signature and BBS+ presentation_header
      reject the modified review.
```

### Attack 3: Forgery (credential偽造)

```
[Eve]    Issuer の秘密鍵 SK_m を知らないまま、適当な BBS+ proof を
         捏造して自分の hpk_eve に紐づけて投稿
         
         → POST /api/registry/publish
         → bbs_verify_proof(π_fake, [hpk_eve,...], PK_m) → FAIL
         → REJECTED
         
表示: 🚫 Forgery detected.
      The proof does not verify against issuer's public key PK_m.
      Only the holder of SK_m can issue valid BBS+ signatures
      that survive proof verification.
```

### Attack 4: Lying about disclosed attribute (v0.2)

```
[Eve]    実 tier=mid の credential を持っている。
         payload.credential_proof.purchase_tier に "high" を捏造して投稿。
         (BBS+ proof は元の tier=mid で生成されている)
         
         → POST /api/v1/reviews
         → verifier rebuilds disclosed_messages = [merchant_id, issued_at, "high"]
         → bbs_blind_proof_verify with disclosed "high"
         → proof hash mismatch (proof was generated for "mid")
         → FAIL
         → REJECTED
         
表示: 🚫 Attribute claim invalid.
      The disclosed purchase_tier does not match the BBS+ signed value.
      Reviewers can choose what to reveal, not what to lie about.
```

---

## 8. 技術スタック (案)

| 層 | 選択肢 | 推し |
|---|---|---|
| 暗号 (BBS+) | Rust + `zkryptium` (BLS12-381) + `ed25519-dalek` | ✅ |
| バックエンド | Rust + `axum` (issuer + registry を1バイナリで serve) | ✅ |
| フロント | SvelteKit + TypeScript | ✅ |
| 暗号呼び出し位置 | (i) クライアント側で WASM 実行 / (ii) サーバ側 API | **(i) WASM** を強く推す |
| デプロイ | 単一 Rust バイナリ (static + API) → Fly.io / Render / VPS | TBD |
| 永続化 | メモリのみ (デモ性優先) | ✅ |

### なぜ WASM (クライアント側暗号) を推すか

- **Cypherpunk的に正しい**: reviewer の wallet 操作 (hsk 生成、commitment、unblind、proof gen) が完全にローカル
- **Eve 攻撃の真実性**: forgery 試行も実際にブラウザ内で起きる、サーバが「検証する」のではなく検証コードがクライアントにある
- **コア crypto crate を共有**: Rust → WASM ビルドで同じコードを Issuer サーバ・Wallet・Verifier 全部で使える
- **デモ動画として強い**: 「全部ブラウザ内で動いてる」が映える

### WASM 性能上の注意

BBS+ は ペアリング暗号 を含むため WASM での処理は BDHKE より重い:
- Mint (blind issuance): ~50ms
- Proof generation: ~100ms
- Proof verification: ~30ms

デモ用途では問題なし。プロダクション wallet では WebWorker で UI ブロック回避推奨。

---

## 9. デプロイ・運用

- 単一 Rust バイナリで以下を serve:
  - `/api/issuer/*` — mint, keysets
  - `/api/registry/*` — publish, list, verify
  - `/` — SPA bundle (HTML/CSS/JS/WASM)
- ストレージ: in-memory (再起動でリセット = 「クリーンな状態でリプレイ」)
- ホスティング: Fly.io free tier or 自前 VPS

---

## 10. 残りの判断事項

| # | 項目 | 候補 |
|---|---|---|
| D1 | フロントフレームワーク | SvelteKit / React (Next) / Solid / Astro |
| D2 | ナレーション方式 | step-by-step ガイドツアー / 各画面に inline explainer / 別ペインで連動表示 |
| D3 | Eve persona の見せ方 | persona 切り替え / 各画面に「Eve として試す」ボタン |
| D4 | デモのスタイル | ダーク / ライト / 1980s ターミナル風 (Cypherpunkらしさ) |
| D5 | デモ動画の長さ目標 | 30秒 / 1分 / 3分 |

---

## 11. Out of scope (今後の展開)

- Moderation layer (display 側のフィルタ、AT Proto labeler 的アプローチ)
- Multi-merchant federation
- Persistent reviewer pseudonym (cross-credential reputation)
- Mobile native wallet
- Issuer key rotation UI
- Real payment processor integration
