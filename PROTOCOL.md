# chaumstar Protocol — v0.3 (draft)

> Anonymous, verifiable reviews from verified purchasers.
> Based on **BBS+ Anonymous Credentials** over BLS12-381 +
> an append-only Merkle **Transparency Log** for the Registry.
>
> - **v0.2** added two reviewer-controlled disclosed attributes
>   (`purchase_tier`, `product_category`).
> - **v0.3** adds a Registry transparency log so readers can verify
>   independently that published reviews are not silently deleted or
>   rewritten. Wire format is incompatible with v0.2 (responses now
>   carry `sth` + `inclusion_proof`).

---

## 1. Overview

chaumstar は「**Issuer の祝福を受けた誰かが書いたレビュー**」を、Issuer・Reader を含む誰にも reviewer の身元を漏らさずに公開・検証可能にするプロトコル。

W3C VC Data Integrity / IRTF CFRG `draft-irtf-cfrg-bbs-signatures` で標準化が進む **BBS+ Signatures** を、決済証明 (verifiable purchaser) というクレデンシャルに転用する。

### なぜ BBS+ か

- **Issuer-side unlinkability**: blind issuance + re-randomizable proof により、Issuer が mint log を持っていても公開レビューと特定 mint event を結びつけられない
- **Selective disclosure**: 必要な属性のみ公開、他は隠蔽
- **Standardized**: W3C VC, IETF CFRG で仕様化進行中、エコシステム互換性
- **Mature implementations**: Rust 等で実装あり (`zkryptium`)

---

## 2. Goals & Non-goals

### Goals

- G1. **検証可能性**: レビューが Issuer の祝福を受けた誰かによって書かれたことを、Reader が独立に暗号的に検証できる
- G2. **匿名性**: Issuer も Reader も、レビューを特定の購入イベント・購入者に紐づけられない (BBS+ blind issuance により本質的に保証)
- G3. **一意性 (linkability)**: 1クレデンシャル = 1レビュー。同一 credential の再使用は検出可能
- G4. **改ざん耐性**: レビュー本文・rating・メタデータの改ざんを検出できる
- G5. **Issuer 中立**: 任意の BBS+ 実装が共存できる。プロトコルは特定の Issuer に依存しない

### Non-goals

- N1. **悪意ある Issuer の防止** — Sybil mint は out-of-scope。Issuer 選定はプロトコル外
- N2. **メタデータからの脱匿名化対策** — 文体・投稿時刻・IPアドレス等は別問題
- N3. **レビュー内容の真正性** — 「美味しかった」が事実か否かは検証しない
- N4. **レビュー報酬の支払い** — 別レイヤー
- N5. **レビューのストレージ・配信** — payload 形式のみ定義、保存・配信は実装者任せ
- N6. **Content moderation** — 誹謗中傷・スパム・違法コンテンツ等のフィルタリングはプロトコルの責務外。display layer のポリシーとして実装される (詳細は § Policy Layer Separation)

---

## 3. Threat Model

| Threat | Mitigation | Status |
|---|---|---|
| 同一クレデンシャルでの二重レビュー | Linkability nullifier (`hpk`) | ✅ in-scope |
| 第三者によるレビュー改ざん | Ed25519 holder signature | ✅ in-scope |
| Issuer による Reviewer の特定 (mint log 有無問わず) | BBS+ blind issuance + ZK proof | ✅ in-scope |
| Reader による Reviewer の特定 | BBS+ ZK proof (signature 非開示) | ✅ in-scope |
| Issuer による架空クレデンシャル発行 | — | ❌ out-of-scope (issuer 選定責任) |
| Registry の検閲・改ざん (process 寿命内) | RFC 6962 流 Merkle transparency log + 署名付き STH + inclusion proof | ✅ in-scope (v0.3) |
| Registry の split-view (同じ tree_size で別 root) | witness cosigning / gossip | ❌ out-of-scope (v1+) |
| Registry の process restart で全消去 | — | ❌ 永続化なし demo の限界 |
| メタデータ経由の脱匿名化 | — | ❌ out-of-scope |
| 競合店による偽レビュー大量投稿 | Issuer の mint policy で対応 | ❌ out-of-scope |
| Reviewer が disclosed 属性で嘘をつく (例: tier=mid なのに high と payload に書く) | BBS+ proof verification が disclosed message hash の不一致を検出 | ✅ in-scope (v0.2) |

### Trust assumptions

- A1. **Issuer は真の購入に対してのみ mint する** — issuer 選定の責任は採用者にある
- A2. **Registry は append-only である** — 透明性ログ / 監査ログで担保
- A3. **Reader は Issuer の真正な公開鍵を入手できる** — 鍵配布は別問題 (Web of Trust / DNS / 各種PKI)
- A4. **BBS+ 実装と BLS12-381 の安全性** — 採用ライブラリの正しさに依存

---

## 4. Actors

| Actor | Role | Holds |
|---|---|---|
| **Issuer** | クレデンシャル発行者 | BBS+ keypair `(SK_m, PK_m)` (per merchant) |
| **Reviewer** | クレデンシャル保持・レビュー公開 | BBS+ signature `σ` + Ed25519 keypair `(hsk, hpk)` |
| **Reader** | レビュー検証 | Issuer 公開鍵 `PK_m`, Registry アクセス |
| **Registry** | nullifier 公開台帳 + transparency log | Append-only Merkle log + Ed25519 keypair (起動時生成) |

---

## 5. Cryptographic Primitive: BBS+ Signatures

### Parameters

- Curve: **BLS12-381** (pairing-friendly)
- BBS+ specification: IRTF `draft-irtf-cfrg-bbs-signatures` (BBS suite with BLS12-381 G1 messages, G2 public keys)
- Hash-to-scalar: BBS spec の `hash_to_scalar` (cf. CRYPTO.md)
- Holder signature: Ed25519 (RFC 8032)

### Mint keysets (per-merchant)

Issuer は merchant ごとに別の BBS+ keypair を持つ。

- For each merchant `m`:
  - secret: `SK_m ∈ Z_r` (BLS12-381 scalar field)
  - public: `PK_m ∈ G_2` (BLS12-381 G2 point, 96 bytes compressed)
  - keyset_id: `kid_m = H(PK_m)[:8]`

Reader は `(issuer_id, merchant_id, keyset_id)` のタプルから `PK_m` を解決する。鍵配布は `https://<issuer>/.well-known/chaumstar/keysets.json` 形式で公開する想定。

### Signed message vector (v0.2)

各 credential は以下のメッセージベクトル `(m₁, m₂, m₃, m₄, m₅)` への BBS+ 署名:

| Index | Message | Source | Disclosure at publish |
|---|---|---|---|
| m₁ | `hpk` (Ed25519 public key, 32B) | reviewer (committed) | **always revealed** (= nullifier) |
| m₂ | `merchant_id` (UTF-8 string → scalar) | issuer | **always revealed** (routing) |
| m₃ | `issued_at` (RFC3339 string → scalar) | issuer | **always revealed** |
| m₄ | `purchase_tier` (`low` / `mid` / `high`) | issuer | **reviewer-controlled** |
| m₅ | `product_category` (`drinks` / `food` / `merch`) | issuer | **reviewer-controlled** |

zkryptium の API では:
- `committed_messages = [hpk]` (holder 側 Pedersen commitment)
- `messages = [merchant_id, issued_at, purchase_tier, product_category]` (issuer reveal)
- `disclosed_commitment_indexes = [0]` 固定 (hpk は nullifier として常に開示)
- `disclosed_indexes` ⊆ `[0, 1, 2, 3]` で、最低 `[0, 1]` (merchant_id / issued_at) を含む

### Disclosure mask

Publish 時、reviewer は `DisclosureMask` で m₄, m₅ を開示するか選ぶ:

```
DisclosureMask {
    disclose_tier: bool,
    disclose_category: bool,
}
```

開示しない属性は BBS+ proof 内で**隠蔽されたまま** (issuer の署名対象だったことは proof で保証されるが、 値は読めない)。 BBS+ の真骨頂。

### なぜ tier / category だけ optional か

- `hpk` は nullifier として必須 → 必ず開示
- `merchant_id` は「どの店のレビューか」を Reader が知る必要 → 必ず開示
- `issued_at` は妥当性チェック (credential 有効期間等) で必要 → 必ず開示
- `purchase_tier` / `product_category` はメタデータ → reviewer が privacy 選択

issuer は `purchase_tier = high` であることを mint 時に attest できるが、 reviewer が「高額顧客と知られたくない」と思えば隠せる。 これが selective disclosure の価値。

---

## 6. Protocol Operations

### 6.1 Mint (blind issuance)

```
Inputs:  MintContext { merchant_id, issued_at, purchase_tier, product_category }
Output:  credential = (hpk, hsk, σ, ctx_attrs, PK_m, kid_m)

1. Reviewer generates fresh Ed25519 keypair (hsk, hpk)
2. Reviewer encodes hpk → scalar m₁
3. Reviewer creates Pedersen-style commitment C_hpk to m₁ with blinding r
4. Reviewer creates PoK of (m₁, r) for the commitment
5. Reviewer sends (C_hpk, PoK, MintContext) to Issuer
6. Issuer verifies purchase context (out-of-scope) and PoK
7. Issuer resolves SK_m for merchant_id, computes blind signature blind_σ
   over (commitment, [merchant_id, issued_at, purchase_tier, product_category])
8. Issuer returns blind_σ (along with PK_m, kid_m)
9. Reviewer unblinds: σ ← unblind(blind_σ, r)
10. Verify σ is valid BBS+ signature on (m₁, m₂, m₃, m₄, m₅) under PK_m
11. Save credential together with the cleartext attribute values
```

詳細な BBS+ blind issuance プロトコルは CRYPTO.md §5 参照。

### 6.2 Publish review

```
Inputs:  (credential, review_body, DisclosureMask)
Output:  payload

1. Resolve disclosed attributes from credential + mask:
       disclosed_tier     = mask.disclose_tier     ? credential.tier     : None
       disclosed_category = mask.disclose_category ? credential.category : None
2. Construct M = canonical_serialize({
       review_body, hpk, keyset_id,
       purchase_tier:    disclosed_tier,
       product_category: disclosed_category
   })
3. σ_ed = Ed25519.sign(hsk, M)
4. Build disclosed_indexes from mask:
       [0, 1]        always
       + [2]         if mask.disclose_tier
       + [3]         if mask.disclose_category
5. Generate fresh BBS+ blind proof:
       π = bbs_blind_proof_gen(
           signature: σ,
           messages:           [merchant_id, issued_at, tier, category],
           committed_messages: [hpk],
           disclosed_indexes,
           disclosed_commitment_indexes: [0],
           pubkey: PK_m,
           presentation_header: H(M)
       )
6. payload = {
       review_body,
       credential_proof: {
           hpk, keyset_id, bbs_proof: π,
           purchase_tier:    disclosed_tier,
           product_category: disclosed_category
       },
       sig: σ_ed
   }
```

### 6.3 Verify

```
Inputs:  (payload, keyset registry, Registry)
Output:  VALID | INVALID(reason)

1. Resolve PK_m from (issuer_id, merchant_id, keyset_id)
2. Recompute M (canonical), verify Ed25519: hpk, M, σ_ed
3. Reconstruct disclosure state from payload.credential_proof:
       disclosed_indexes        = [0, 1]
                                  + [2] if purchase_tier is Some
                                  + [3] if product_category is Some
       disclosed_messages       = [merchant_id, issued_at,
                                   (tier if present), (category if present)]
4. Verify BBS+ blind proof:
       bbs_blind_proof_verify(
           proof: π,
           disclosed_messages,
           disclosed_committed_messages: [hpk],
           disclosed_indexes,
           disclosed_commitment_indexes: [0],
           pubkey: PK_m,
           presentation_header: H(M)
       )
5. Check Registry: hpk not in spent_set
6. If all pass: append hpk to Registry.spent_set; return VALID
```

> If reviewer lies — e.g. payload claims `purchase_tier = "high"` but the
> credential's signed tier is `"mid"` — step 4 fails because the disclosed
> message hash does not match what BBS+ vouched for. The reviewer can choose
> what to reveal but cannot lie about revealed values.

### 6.4 Linkability check (Registry side)

```
- hpk が初出 → VALID 承認 → spent_set に追加
- hpk が既出 → INVALID(double_review)
```

### 6.5 Why issuer cannot link mint to publish

詳細は CRYPTO.md §6 参照。要点:

- Mint 時、Issuer は `C_hpk` (Pedersen commitment) のみを見る — `hpk` 自体は知らない
- Mint log には `(C_hpk_i, blind_σ_i, purchaser_info_i)` が残る
- Publish 時、`hpk` は公開、`σ` は BBS+ proof 内で隠蔽
- Issuer が log と publish を紐づけるには、各 log entry について `r_i s.t. C_hpk_i = h₁^hpk · g'^r_i` を解く必要があり、これは離散対数問題 (hard)
- BBS+ proof の re-randomization により、signature `σ` も log の `blind_σ_i` と直接照合不可

---

## 7. Data Formats (draft)

### Credential (Reviewer 保持、private)

```json
{
  "version": "chaumstar/0.2",
  "hpk": "<ed25519 pubkey, 32B hex>",
  "hsk": "<ed25519 privkey, 32B hex>  // SECRET, never publish",
  "signature": "<BBS+ signature, 80B hex>",
  "issuer_id": "<string>",
  "merchant_id": "<string>",
  "purchase_tier": "low|mid|high",
  "product_category": "drinks|food|merch",
  "keyset_id": "<hex 8B>",
  "issuer_pubkey": "<BLS12-381 G2 compressed, 96B hex>",
  "issued_at": "<RFC3339>"
}
```

`signature` は **wallet 内に秘匿** され、外部に出さない。Publish 時には ZK proof として変換される。`purchase_tier` / `product_category` も wallet で平文保存される (BBS+ で隠せるのは proof 公開時の値であって、 credential 内には平文で残る)。

### Review payload (公開)

```json
{
  "version": "chaumstar/0.2",
  "review_body": {
    "text": "<string>",
    "rating": 1,
    "merchant_id": "<string>",
    "issuer_id": "<string>",
    "issued_at": "<RFC3339>",
    "timestamp": "<RFC3339>"
  },
  "credential_proof": {
    "hpk": "<ed25519 pubkey, 32B hex>",
    "keyset_id": "<hex 8B>",
    "bbs_proof": "<BBS+ proof, ~370-450B hex>",
    "purchase_tier":    null,
    "product_category": null
  },
  "sig": "<ed25519 sig, 64B hex>"
}
```

`purchase_tier` / `product_category` は **`null` なら隠蔽**、 string なら **その値を BBS+ proof で開示している** ことを示す。 検証側はこの2つから disclosed_indexes を構築する (§6.3)。

`sig` の署名対象 `M` の構造は `CRYPTO.md` §7 で規定。`bbs_proof` は `presentation_header = H(M)` で生成されるため、レビュー本文 + 開示属性と暗号的に結合され、本文や開示値の改ざんで proof 検証失敗する。

---

## 7.5 Transparency Log (v0.3)

Registry が公開済みレビューを **削除 / 改ざん / 履歴書き換え** していないことを、
Reader が独立に検証可能にする。 RFC 6962 (Certificate Transparency) 流の append-only Merkle log を採用。

### 構成

| 要素 | 役割 |
|---|---|
| **Merkle log** | Registry が保持。 leaf = `SHA-256(0x00 ‖ canonical_payload)`。 内部 node = `SHA-256(0x01 ‖ left ‖ right)` |
| **Registry keypair** | 起動時に Ed25519 生成。 `/registry-key` で公開。 STH 署名用 |
| **STH (Signed Tree Head)** | `{tree_size, root_hash, timestamp, sig}`。 各 publish 後に新 STH を発行 |
| **InclusionProof** | `{leaf_index, tree_size, path[]}`。 1 review が特定 tree に含まれることの証明 |

### 検証ステップ (Reader)

```
1. /api/v1/registry-key で Registry pubkey を取得 (1 回)
2. /api/v1/reviews で payloads + sth + inclusion_proofs を取得
3. 各 payload について:
   a. STH 署名検証 (Ed25519, Registry pubkey 使用)
   b. leaf = SHA-256(0x00 ‖ canonical_payload)
   c. Merkle path を辿って root を再計算
   d. root が STH.root_hash と一致するか確認
4. 既存の BBS+ proof + Ed25519 sig 検証も並行 (§6.3)
```

すべて pass すれば 「この review は Registry が **その時点で** 自身の log に commit した」 が証明される。

### 何が防げるか

| 攻撃シナリオ | この設計の応答 |
|---|---|
| Registry が hpk_X を「未登録」と嘘 | Reader が以前受領した inclusion_proof + STH を提示すれば嘘がバレる |
| Registry が後から hpk_X を消す | tree_size が下がるか root が変わる → 古い STH との不整合で発覚 |
| Registry が hpk_X の payload を書き換え | leaf_hash が変わる → 古い inclusion_proof が無効化される → 不整合で発覚 |

### 残る限界

| 限界 | 理由 |
|---|---|
| **Split-view attack** (異なる reader に違う tree を見せる) | 1 reader だけでは検出不能。 witness cosigning / gossip が要る。 v1+ |
| **Process restart で全部リセット** | demo 永続化なし前提。 Registry pubkey も変わるので過去の STH が orphan に |
| **長期 audit** | 永続化なし demo では実質意味なし。 持続させるなら独立 monitor + 外部 log が要る |

### Wire format additions

新エンドポイント:

```
GET  /api/v1/registry-key       → { public_key: <ed25519 pubkey, hex 32B> }
GET  /api/v1/sth                → Sth
```

既存エンドポイントの response 拡張:

```
POST /api/v1/reviews            → { payload, inclusion_proof, sth }
GET  /api/v1/reviews            → { reviews: [{ payload, inclusion_proof }], sth }
GET  /api/v1/reviews/{hpk}      → { payload, inclusion_proof, sth }
```

`Sth` / `InclusionProof` の構造は `CRYPTO.md` §8 を参照。

---

## 8. Open Design Decisions

| # | Item | Tentative |
|---|---|---|
| 1 | Merchant binding | ✅ **per-merchant BBS+ keyset** (各 merchant が独立 SK/PK) |
| 2 | Credential identifier | ✅ **`hpk` (Ed25519 pubkey)**、message vector の `m₁` として署名対象 |
| 3 | Issuance mode | ✅ **Blind issuance** (Issuer は hpk を mint 時に見ない) |
| 4 | Disclosure 戦略 | ✅ **v0.2**: `hpk` / `merchant_id` / `issued_at` は必須開示、 `purchase_tier` / `product_category` は reviewer-controlled |
| 5 | Registry 実装 | MVP: HTTP+DB single server / v1: Merkle transparency log + federation |
| 6 | Issuer key rotation | 後回し (epoch ベース想定) |
| 7 | クレデンシャルの失効 | 不要 (1-use) |
| 8 | Reviewer UX | MVP は Web wallet |
| 9 | Canonical serialization | ✅ **JCS** (RFC 8785) |
| 10 | BBS+ library 選択 | **zkryptium** 第一候補、arkworks 自作も検討余地 |

---

## 9. Policy Layer Separation

chaumstar が暗号的に保証するのは **「verified purchaser」のみ**。レビュー内容の **真正性・適切性・合法性** は保証しない。

これは設計上の限界ではなく **明示的な責務分離**:

| Layer | Responsibility | Provided by |
|---|---|---|
| **Protocol** (chaumstar) | 「Issuer の祝福を受けた誰かが書いた」を暗号的に証明 | 本仕様 |
| **Display** (review viewer apps) | 内容の表示・フィルタリング・ランキング・通報受付 | 実装者・ユーザコミュニティ |
| **Moderation** (subscribable labelers) | ポリシー判定・違反検出・カテゴリ分類 | 第三者 (政府・NPO・コミュニティ・AI service) |
| **Legal** | 違法コンテンツへの対処・発信者特定 | 司法・行政 |

### Design rationale

「真正性 (誰が書いたか)」と「適切性 (何を書いたか)」を同じレイヤーで扱うと、**必ず検閲権力が一か所に集中する**。両者を切り離すことで:

- 暗号レイヤーの純度を保つ (Issuer/プラットフォーム横断で検証可能)
- ポリシーを差し替え可能にする (国・コミュニティ・個人ごとの判断を許容)
- 検閲・revocation を Issuer の権力にしない (Issuer はレビュー後の判断に介入できない)

これは Bitcoin (プロトコル中立・ウォレット側で表示判断) や AT Protocol (labeler 購読モデル) と同じ思想。

### Display layer に期待される機能

(プロトコル外、実装者任意)

- レビュー内容のフィルタリング (NG ワード, AI 分類, 通報集約)
- カテゴリ判定 (賛辞/批判/オフトピック)
- ユーザ選択可能な moderation policy (subscribable labelers)
- 違法コンテンツの非表示・行政対応

### What chaumstar *cannot* do

- レビュー削除 (registry は append-only)
- Reviewer 特定 (anonymous credential)
- 内容の評価 (semantic 判断は範囲外)

これらを「やりたい場合」は別レイヤーで実装し、chaumstar の payload は不変のまま扱う。

---

## 10. Out of scope (将来検討)

- Multi-issuer aggregation (横断レビューサイト)
- Reviewer reputation across pseudonymous identities
- Cross-merchant linkability (現状: per-credential nullifier のみ)
- Encrypted reviews (reader 限定公開)
- Threshold issuance (M-of-N mint operators)
- Selective disclosure ベースの additional attribute (購入金額, 商品 SKU, 来店回数, etc.)

---

## 11. References

- IRTF CFRG draft: BBS Signatures — https://datatracker.ietf.org/doc/draft-irtf-cfrg-bbs-signatures/
- W3C VC Data Integrity BBS Cryptosuite — https://www.w3.org/TR/vc-di-bbs/
- Boneh, Boyen, Shacham (2004). "Short Group Signatures"
- Camenisch & Lysyanskaya — Anonymous Credentials
- RFC 8032: EdDSA / Ed25519
- RFC 8785: JSON Canonical Serialization (JCS)
- `zkryptium` Rust crate — https://github.com/Cybersecurity-LINKS/zkryptium
- BLS12-381 curve specification
