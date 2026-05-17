# chaumstar Protocol — v0 (draft)

> Anonymous, verifiable reviews from verified purchasers.
> Based on **BBS+ Anonymous Credentials** over BLS12-381.

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
| Registry の検閲・改ざん | 透明性ログ実装で監査可能 | △ 設計上は対応可能、MVP は単一サーバ |
| メタデータ経由の脱匿名化 | — | ❌ out-of-scope |
| 競合店による偽レビュー大量投稿 | Issuer の mint policy で対応 | ❌ out-of-scope |

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
| **Registry** | nullifier 公開台帳 | Append-only set of used `hpk` |

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

### Signed message vector

各 credential は以下のメッセージベクトル `(m₁, m₂, m₃)` への BBS+ 署名:

| Index | Message | Disclosure at publish |
|---|---|---|
| m₁ | `hpk` (Ed25519 public key, 32B) | revealed (= nullifier) |
| m₂ | `merchant_id` (UTF-8 string → scalar) | revealed |
| m₃ | `issued_at` (RFC3339 string → scalar) | revealed |

Reviewer は presentation 時に全 message を revealed として ZK proof を生成する。Signature `σ` 自体は **公開されない** (BBS+ proof 内で隠蔽)。

### Why all messages are revealed

本プロトコルでは selective disclosure を使わず全 message を revealed として presentation する。理由:

- `hpk` は nullifier として必要 (revealed)
- `merchant_id` は routing/表示のため revealed
- `issued_at` は妥当性確認のため revealed

それでも privacy は保たれる: **BBS+ proof は signature σ 自体を隠蔽する** ので、issuer は mint log の `blind_sig_i` と公開 proof を紐づけられない (詳細 CRYPTO.md §6)。

---

## 6. Protocol Operations

### 6.1 Mint (blind issuance)

```
Inputs:  (Issuer purchase context, Reviewer, merchant_id)
Output:  credential = (hpk, hsk, σ, PK_m, kid_m, issuer_id, merchant_id, issued_at)

1. Reviewer generates fresh Ed25519 keypair (hsk, hpk)
2. Reviewer encodes hpk → scalar m₁
3. Reviewer creates Pedersen-style commitment C_hpk to m₁ with blinding r
4. Reviewer creates PoK of (m₁, r) for the commitment
5. Reviewer sends (C_hpk, PoK, merchant_id, issued_at) to Issuer
6. Issuer verifies purchase context (out-of-scope) and PoK
7. Issuer resolves SK_m for merchant_id, computes blind signature blind_σ
8. Issuer returns blind_σ (along with PK_m, kid_m)
9. Reviewer unblinds: σ ← unblind(blind_σ, r)
10. Verify σ is valid BBS+ signature on (m₁, m₂, m₃) under PK_m
11. Save credential
```

詳細な BBS+ blind issuance プロトコルは CRYPTO.md §5 参照。

### 6.2 Publish review

```
Inputs:  (credential, review_text, rating)
Output:  payload

1. Construct M = canonical_serialize({
       review_text, rating,
       merchant_id, issuer_id, issued_at,
       hpk, keyset_id
   })
2. σ_ed = Ed25519.sign(hsk, M)
3. Encode messages m₁ = hpk, m₂ = merchant_id, m₃ = issued_at
4. Generate fresh BBS+ proof:
       π = bbs_create_proof(
           signature: σ,
           messages: [m₁, m₂, m₃],
           disclosed_indices: [1, 2, 3],     // all revealed
           pubkey: PK_m,
           presentation_header: H(M)         // domain-binding the proof to this review
       )
5. payload = {
       review_body: { text, rating, merchant_id, issuer_id, issued_at, timestamp },
       credential_proof: {
           hpk, keyset_id, bbs_proof: π
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
3. Verify BBS+ proof:
       bbs_verify_proof(
           proof: π,
           disclosed_messages: [hpk, merchant_id, issued_at],
           pubkey: PK_m,
           presentation_header: H(M)
       )
4. Check Registry: hpk not in spent_set
5. If all pass: append hpk to Registry.spent_set; return VALID
```

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
  "version": "chaumstar/0.1",
  "hpk": "<ed25519 pubkey, 32B hex>",
  "hsk": "<ed25519 privkey, 32B hex>  // SECRET, never publish",
  "signature": "<BBS+ signature, ~112B hex>",
  "issuer_id": "<string>",
  "merchant_id": "<string>",
  "keyset_id": "<hex 8B>",
  "issuer_pubkey": "<BLS12-381 G2 compressed, 96B hex>",
  "issued_at": "<RFC3339>"
}
```

`signature` は **wallet 内に秘匿** され、外部に出さない。Publish 時には ZK proof として変換される。

### Review payload (公開)

```json
{
  "version": "chaumstar/0.1",
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
    "bbs_proof": "<BBS+ proof, ~300-500B hex>"
  },
  "sig": "<ed25519 sig, 64B hex>"
}
```

`sig` の署名対象 `M` の構造は `CRYPTO.md` §7 で規定。`bbs_proof` は `presentation_header = H(M)` で生成されるため、レビュー本文と暗号的に結合され、本文改ざんで proof 検証失敗する。

---

## 8. Open Design Decisions

| # | Item | Tentative |
|---|---|---|
| 1 | Merchant binding | ✅ **per-merchant BBS+ keyset** (各 merchant が独立 SK/PK) |
| 2 | Credential identifier | ✅ **`hpk` (Ed25519 pubkey)**、message vector の `m₁` として署名対象 |
| 3 | Issuance mode | ✅ **Blind issuance** (Issuer は hpk を mint 時に見ない) |
| 4 | Disclosure 戦略 | ✅ **全 message を revealed** (selective disclosure は将来用) |
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
