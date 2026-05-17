# chaumstar Cryptography — v0 (draft)

> 実装者向けの暗号仕様。プロトコル概要は `PROTOCOL.md` を、デモは `DEMO.md` を参照。

---

## 1. Scope

このドキュメントは以下を規定する:

- 曲線・ハッシュ関数等のパラメータ
- BBS+ Signatures の仕様参照と chaumstar 固有の設定
- Blind issuance の詳細
- Presentation proof (BBS+ proof of knowledge) の詳細
- Ed25519 holder signature の使用
- Canonical serialization (JCS) の chaumstar 固有ルール
- Domain separation tags (DST)
- 既知のセキュリティ上の限界

---

## 2. Notation

| 記号 | 意味 |
|---|---|
| `G₁`, `G₂`, `Gₜ` | BLS12-381 のグループ |
| `r` | BLS12-381 の scalar field 位数 (素数) |
| `e(·,·)` | type-3 ペアリング `G₁ × G₂ → Gₜ` |
| `[s]P` | scalar `s` による曲線点 `P` の scalar mult |
| `‖` | byte concatenation |
| `H_s(·)` | hash-to-scalar (BBS spec) |
| `H(·)` | SHA-256 |
| `||M||` | canonical serialization (JCS) of `M` |

---

## 3. Parameters

### 3.1 Curve: BLS12-381

- ペアリング: type-3, `e: G₁ × G₂ → Gₜ`
- `G₁` 圧縮: 48 bytes
- `G₂` 圧縮: 96 bytes
- Scalar (in `Z_r`): 32 bytes (`r ≈ 2^254`)

### 3.2 BBS suite

採用: **IRTF CFRG `draft-irtf-cfrg-bbs-signatures` の "BLS12-381-SHA-256" ciphersuite**

具体的には:
- Signature in `G₁`
- Public key in `G₂`
- Hash: SHA-256
- Hash-to-curve: `BLS12381G1_XMD:SHA-256_SSWU_RO_` (RFC 9380)
- Hash-to-scalar: BBS spec §4.4

→ `chaumstar` は CFRG 仕様を**そのまま**使用。独自パラメータは導入しない。

### 3.3 Encodings

| 対象 | Encoding | Size |
|---|---|---|
| BLS12-381 G₁ point | compressed (BBS spec §1.2) | 48 bytes |
| BLS12-381 G₂ point | compressed (BBS spec §1.2) | 96 bytes |
| BLS12-381 scalar | big-endian | 32 bytes |
| BBS+ signature | (A: G₁, e: scalar) | 80 bytes |
| BBS+ proof | (Abar, Bbar, D, e, ...) variable | ~300-400 bytes |
| Ed25519 public key | RFC 8032 | 32 bytes |
| Ed25519 signature | RFC 8032 | 64 bytes |
| バイナリの JSON 表現 | lowercase hex | 2× バイト長 |

---

## 4. Message vector definition (v0.2)

各 credential は次の **5-message vector** に BBS+ 署名:

| Index | Name | Source | Type | Encoding for `H_s` | Disclosure |
|---|---|---|---|---|---|
| 1 | `hpk` | reviewer (committed) | Ed25519 pubkey (32B) | raw 32 bytes | always revealed (nullifier) |
| 2 | `merchant_id` | issuer | UTF-8 string | UTF-8 bytes | always revealed |
| 3 | `issued_at` | issuer | RFC3339 timestamp | UTF-8 bytes | always revealed |
| 4 | `purchase_tier` | issuer | `low` / `mid` / `high` | UTF-8 bytes of the literal | reviewer-controlled |
| 5 | `product_category` | issuer | `drinks` / `food` / `merch` | UTF-8 bytes of the literal | reviewer-controlled |

zkryptium 内では:
- `committed_messages = [hpk]`
- `messages = [merchant_id, issued_at, purchase_tier, product_category]` (issuer-known)
- `disclosed_commitment_indexes = [0]` 固定
- `disclosed_indexes ⊆ [0,1,2,3]`、必ず `0` と `1` を含む

各メッセージは BBS spec の `messages_to_scalars` 関数で `Z_r` の scalar に変換される:

```
m_i_scalar = hash_to_scalar(message_i_bytes, dst=...)
```

---

## 5. BBS+ Blind Issuance Protocol

### 5.1 設定

Issuer は merchant ごとに BBS+ keypair を持つ:
- `SK_m ∈ Z_r`
- `PK_m = [SK_m]·G₂_base ∈ G₂`
- `kid_m = H(PK_m_compressed)[:8]`

加えて、blind issuance のための public generator `H_1, H_2, H_3 ∈ G₁` を BBS spec の `create_generators` で導出する (deterministic from issuer context)。

### 5.2 Reviewer の commitment 生成

```
Inputs:  hpk (32B), MintContext { merchant_id, issued_at, purchase_tier, product_category }
Outputs: (commitment, blinding, pok_commitment)

1. m₁ = hash_to_scalar(hpk)

2. ρ ← random scalar in Z_r                     // blinding
3. C_blind = [m₁]H₁ + [ρ]H₀                     // Pedersen-style commitment on hpk only
4. NIZK proof of knowledge of (m₁, ρ):
   pok_commit = create_pok(C_blind, [m₁, ρ], [H₁, H₀])

5. Send (C_blind, pok_commit, MintContext) to Issuer
```

`purchase_tier` / `product_category` は **issuer 側で attest される値** であり commitment には含まれない。 Reviewer は受け入れるだけ (希望値を request 時に送るが、 issuer の judgement で上書きされ得る — 実装方針 §5.3 参照)。

### 5.3 Issuer の blind sign

```
Inputs:  (C_blind, pok_commit, MintContext, SK_m)
Outputs: blind_signature

1. Verify pok_commit                                       // 必須
2. Verify purchase context (out-of-scope: 会計実態の確認)

3. m₂ = hash_to_scalar(merchant_id_bytes)
4. m₃ = hash_to_scalar(issued_at_bytes)
5. m₄ = hash_to_scalar(purchase_tier_bytes)                // "low" / "mid" / "high"
6. m₅ = hash_to_scalar(product_category_bytes)             // "drinks" / "food" / "merch"

7. blind_σ = bbs_blind_sign(
       SK_m, PK_m,
       commitment: C_blind,
       known_messages: [(2, m₂), (3, m₃), (4, m₄), (5, m₅)],
       generators: [H₁, H₂, H₃, H₄, H₅]
   )

8. Return blind_σ = (A, e)
```

`purchase_tier` / `product_category` は **issuer が judgement で決める**。 reviewer の希望値はあくまで suggestion。 これにより「reviewer が tier=high と勝手に名乗る」攻撃を防ぐ (BBS+ proof は issuer が署名した値しか検証時に通らない)。

### 5.4 Reviewer の unblind

```
Inputs:  (blind_σ = (A_blind, e), ρ, generators)
Outputs: σ (clean BBS+ signature on (m₁, ..., m₅))

1. A = A_blind                                  // BBS+ では A は変化しない
2. σ = (A, e)
3. Verify locally: bbs_verify(σ, [m₁, m₂, m₃, m₄, m₅], PK_m)
4. Save σ + cleartext (merchant_id, issued_at, tier, category) in credential
```

(Pedersen commitment 形式の場合、unblind は単に `ρ` を覚えておく操作)

### 5.5 セキュリティ性質

- **Hiding**: Issuer は `C_blind` から `m₁` (= hpk) を回復できない (DL 仮定)
- **Binding**: ρ と m₁ が一意に commit される
- **Signature on hidden message**: 標準 BBS+ verification が unblind 後に成立
- **No leakage**: blind_σ は publish 時には公開されず、proof に変換される

---

## 6. Presentation Proof

### 6.1 目的

Reviewer は手元の `σ` を直接公開せず、以下を ZK で証明:

```
Statement: ∃ σ such that bbs_verify(σ, [m₁..m₅], PK_m) = true
           ∧ disclosed subset is revealed at chosen values
           ∧ undisclosed subset stays hidden
```

開示集合は最小 `{m₁ (hpk), m₂ (merchant_id), m₃ (issued_at)}`、 オプションで `m₄ (tier)` / `m₅ (category)` を追加。

### 6.2 生成 (Reviewer)

```
Inputs:  σ, [m₁..m₅], PK_m, presentation_header, DisclosureMask
Output:  proof π

1. Build disclosed_indexes from mask:
       base = [0, 1]                      (merchant_id, issued_at — always)
       + [2] if mask.disclose_tier
       + [3] if mask.disclose_category
   disclosed_commitment_indexes = [0]    (hpk — always)

2. proof = bbs_blind_proof_gen(
       signature: σ,
       header: HEADER,                   // chaumstar protocol-level header
       presentation_header: H(M_jcs),    // ← レビュー本文 + 開示属性への bind
       messages:           [m₂, m₃, m₄, m₅],
       committed_messages: [m₁],
       disclosed_indexes,
       disclosed_commitment_indexes: [0],
       PK: PK_m
   )

3. proof は乱数で randomize されている。
   同じ σ から複数回 proof を作っても unlinkable。
```

`presentation_header = H(M_jcs)` により、proof はレビュー本文 + 開示属性に **暗号的に結合** される (本文や開示値を書き換えると proof verify が失敗)。

### 6.3 検証 (Reader)

```
Inputs:  (payload, PK_m)
Output:  VALID | INVALID

1. Reconstruct M_jcs from payload, compute presentation_header = H(M_jcs)
2. Build disclosed_indexes from payload.credential_proof:
       [0, 1]
       + [2] if payload.credential_proof.purchase_tier is Some
       + [3] if payload.credential_proof.product_category is Some
3. Build disclosed_messages in same order:
       [merchant_id, issued_at,
        (tier if disclosed),
        (category if disclosed)]
4. result = bbs_blind_proof_verify(
       proof,
       disclosed_messages,
       disclosed_committed_messages: [hpk],
       disclosed_indexes,
       disclosed_commitment_indexes: [0],
       header: HEADER,
       presentation_header,
       PK: PK_m
   )
5. Return result
```

> **Reviewer が disclosed 値で嘘をついても通らない**: payload で
> `purchase_tier = "high"` を claim しても、 BBS+ proof は credential 内の
> 実 tier (例: `mid`) で生成されている → step 4 で hash 不一致を検出して
> INVALID。 reviewer は **「開示するか / 隠すか」しか選べず、 値の改竄は不可**。

### 6.4 Why issuer cannot link mint to publish

Mint 時、Issuer の log:
```
log[i] = (C_blind_i, pok_commit_i, merchant_id_i, issued_at_i,
          blind_σ_i = (A_blind_i, e_i),
          purchaser_info_i, timestamp_i)
```

Publish 時、Issuer が見える public payload:
```
payload = (review_body, hpk, merchant_id, issued_at, π_bbs, σ_ed)
```

Issuer が linking を試みる:

**Method 1**: `hpk` と `C_blind_i` を直接照合
- `C_blind_i = [hash_to_scalar(hpk)]H₁ + [ρ_i]H₀`
- 与えられた `hpk`, `H₁`, `H₀` から `[ρ_i]H₀ = C_blind_i - [m₁]H₁` を計算可能
- でも `ρ_i` 自体を取り出すには DL on `H₀` を解く必要 → 困難

**Method 2**: BBS proof π から signature を抽出
- `π = (Abar, Bbar, D, e_proof, c, ...)` は σ の randomization
- σ を直接含まず、ZK 性質により σ への計算的束縛なし
- 再度同じ σ から作っても全く異なる π になる

**Method 3**: `A_blind_i` と π 内の `A_bar` 等を直接比較
- `A_bar = [r]A` where `r` は presentation 時の乱数
- log の `A_blind_i` と比較するには `r` を知る必要 → 攻撃者には不可

→ いずれの method も離散対数や proof の randomization により遮断される。**Issuer に mint log があっても、特定 publish との link は計算困難**。

### 6.5 限界 (out-of-cryptographic-scope)

純粋な暗号的紐付けは防げるが、以下は別レイヤーの問題:

- **タイミング相関**: mint event の timestamp と publish の timestamp が近すぎる場合、統計的に紐付けられる可能性。Wallet 側で publish を意図的に遅延させる、batching する等で緩和
- **メタデータ相関**: IP アドレス, User-Agent, network egress 等。Tor 等の anonymity network 利用が必要
- **コンテンツ漏洩**: 文体 / 個人を特定可能な記述

---

## 7. Canonical Serialization (JCS)

### 7.1 採用方式

**RFC 8785 (JSON Canonical Serialization, JCS)**

Rust 実装は `serde_jcs` crate を想定。

### 7.2 chaumstar 固有ルール

- すべての JSON は UTF-8、NFC 正規化済み文字列
- バイナリは lowercase hex 文字列
- タイムスタンプは `RFC3339` の Z 表記 (`"2026-05-17T12:34:56Z"`)、秒精度
- 整数は I-JSON 範囲 (`[-(2^53-1), 2^53-1]`) のみ使用
- floating point は使用しない (rating は integer 1-5)

### 7.3 署名対象 `M` の構造 (v0.2)

Reviewer の Ed25519 sig 対象 (BBS+ presentation_header のソースでもある):

```json
{
  "v": "chaumstar/0.2",
  "type": "review",
  "review_body": {
    "text": "...",
    "rating": 5,
    "merchant_id": "...",
    "issuer_id": "...",
    "issued_at": "2026-05-17T12:34:56Z",
    "timestamp": "2026-05-17T13:00:00Z"
  },
  "credential": {
    "hpk": "<hex>",
    "keyset_id": "<hex>"
  },
  "disclosure": {
    "purchase_tier":    null,
    "product_category": "drinks"
  }
}
```

`disclosure` の各フィールドは **`null` なら隠蔽**、 値があれば **その値が BBS+ で開示されている** ことを示す。 `M` に含めることで:

1. Ed25519 sig が開示状態を bind (reviewer が後から開示状態を書き換えられない)
2. presentation_header = `SHA256(M_jcs)` 経由で BBS+ proof も bind

これを JCS 直列化したバイト列 `M_jcs` を:
- Ed25519 sig: `σ_ed = Ed25519.sign(hsk, M_jcs)`
- BBS+ presentation_header: `presentation_header = SHA256(M_jcs)`

の両方に使う。これにより `bbs_proof` と `σ_ed` が同一の本文に結合される。

---

## 7.5 Transparency Log (v0.3)

### 7.5.1 Hash construction (RFC 6962 流)

| What | Construction |
|---|---|
| Leaf hash | `leaf_hash(payload) = SHA-256(0x00 ‖ canonical_serialize(payload))` |
| Internal node hash | `node_hash(L, R) = SHA-256(0x01 ‖ L ‖ R)` |
| Empty tree root | `SHA-256(empty) = e3b0c4...` (RFC 6962 §2.1) |

奇数 tree の右端 leaf は padding せず、 そのまま (途中段で対なしの場合は次の段にそのまま持ち上げる) — RFC 6962 §2.1 と一致。

### 7.5.2 Sth (Signed Tree Head) の canonical 形式

STH の署名対象 `M_sth` は JCS 直列化:

```json
{
  "v": "chaumstar/0.3",
  "type": "sth",
  "tree_size": <u64>,
  "root_hash": "<hex 32B>",
  "timestamp": "<RFC3339 Z 秒精度>"
}
```

`sig = Ed25519.sign(registry_sk, M_sth_jcs)` で 64 バイト。

### 7.5.3 InclusionProof

`InclusionProof = (leaf_index: u64, tree_size: u64, path: Vec<[u8;32]>)`

検証:

```
1. h = leaf_hash(payload)
2. i = leaf_index
3. for sibling in path:
       if i is even:
           h = node_hash(h, sibling)
       else:
           h = node_hash(sibling, h)
       i = i >> 1
4. assert h == sth.root_hash
```

`tree_size` は path 長を決定するために必要 (途中 padding なし設計のため、 単純な `log2(N)` では不足する場合あり)。

### 7.5.4 Registry keypair

- 起動時に Ed25519 (`SigningKey::generate(OsRng)`) で生成
- 過去の STH を信頼するには `/api/v1/registry-key` で取得した pubkey が必須
- **process restart で鍵が変わる** → 過去 STH はすべて orphan (検証不能)
- demo の限界、 production では epoch + 重複期間で rotation 設計が要

### 7.5.5 既知の限界

| 限界 | 説明 |
|---|---|
| Split-view | 単一 reader では検出不能 |
| 永続化なし | restart で全消去 |
| Consistency proof 未実装 | 異なる STH 間の整合性自動検証はなし (v1+) |
| Non-inclusion proof 未実装 | 「hpk_X が含まれていない」の cryptographic 証明はなし |

---

## 8. Domain Separation Tags

chaumstar 固有 DST は最小限。BBS spec 内の DST は ciphersuite 標準のまま:

| 用途 | DST source |
|---|---|
| BBS hash-to-scalar | BBS spec の ciphersuite ID 由来 |
| BBS hash-to-curve (G₁) | `BLS12381G1_XMD:SHA-256_SSWU_RO_` + ciphersuite-specific DST |
| BBS create_generators | BBS spec §4.1 |
| Ed25519 sig | (Ed25519 自体に DST 概念なし、JCS bytes を直接署名) |
| presentation_header (chaumstar 固有) | `SHA256(M_jcs)` をそのまま使用 |

---

## 9. Ed25519 Holder Signature

### 9.1 採用

**Ed25519 (RFC 8032)** をそのまま使用。

- `hsk`: 32 bytes seed
- `hpk`: 32 bytes public key
- 署名: `σ_ed = Ed25519.sign(hsk, M_jcs)`
- 検証: `Ed25519.verify(hpk, M_jcs, σ_ed)`

### 9.2 BBS+ との関係

- `hpk` は BBS+ message vector の `m₁` として署名対象
- 同時に Reader が直接見る "署名キー" として機能
- `H_s(hpk)` が m₁ scalar、`hpk` 自体は payload で revealed

→ Ed25519 (Edwards 曲線) と BLS12-381 (BLS 曲線) が **共存**。実装上は別 crate を使い分け。

### 9.3 なぜ Ed25519 を BBS+ の上に重ねるか

BBS+ proof は「issuer が誰かに credential を発行した」を示すが、 **「いま投稿した本文の作成者がその誰か本人である」** という証拠は別途必要。

具体的には、 BBS+ proof だけだと:
- credential を盗んだ Eve が、Alice の credential で別本文を投稿し得る (proof は σ を知らなくても生成不可なので、σ を持ってないと無理だが)
- σ を持つ者でも、本文と presentation_header を別に作れば

を防ぐため Ed25519 で本文を署名し、`hpk` で結びつける。

---

## 10. Security Considerations

### 10.1 既知の限界

| 限界 | 影響 | 対策 |
|---|---|---|
| `zkryptium` (BBS+ impl) の audit 状況 | bug が脆弱性に直結 | code review、test vectors との一致確認、可能なら独立実装と cross-check |
| BLS12-381 の量子耐性なし | 将来 quantum-safe 移行必要 | v2 検討 (ML-DSA + BBS-quantum-safe 等) |
| BBS+ blind issuance spec の draft 段階 | spec 変更リスク | CFRG draft 進行を追従、initial deploy 後の migration plan |
| `hsk`, `σ` の wallet 漏洩 | credential 完全奪取 | wallet 実装責任 (encrypted storage) |
| Registry compromise | 任意の hpk を "未使用" と詐称、または "使用済み" と検閲 | transparency log + 監査 (v1) |
| タイミング/メタデータ脱匿名化 | publish timing で mint event 推定可能 | Wallet 側で publish 遅延、Tor 経由推奨 |

### 10.2 Trust on First Use (TOFU) for Issuer pubkey

Issuer の `PK_m` をどう信頼するかはプロトコル外。想定する配布経路:

- `https://<issuer>/.well-known/chaumstar/keysets.json`
- DNSSEC + TLS
- 将来: Certificate Transparency 風の pubkey transparency log

### 10.3 リプレイ攻撃

公開レビューは append-only registry に登録され、`hpk` で重複検出されるため、リプレイは無効 (registry レベルで reject)。`presentation_header` で本文に結合されているので、proof のリプレイ + 別本文への流用も検出される。

### 10.4 タイミング攻撃

すべての scalar 演算は定数時間実装を使用 (採用ライブラリのデフォルトに依存)。ペアリングは定数時間が望ましいが、verify 時のみなので攻撃面は限定的。

### 10.5 Pedersen commitment の安全性

`C_blind = [m₁]H₁ + [ρ]H₀` のセキュリティは:
- Computational hiding: `m₁` を回復するには DL を解く必要
- Binding: `(m₁, ρ)` と `(m₁', ρ')` が同じ C_blind を生成するには DL を解く必要

両性質とも BLS12-381 G₁ 上の DL hardness に帰着。

---

## 11. Open Items (v0 → v1)

| 項目 | v0 状態 | v1 検討 |
|---|---|---|
| Registry transparency | ✅ v0.3 で Merkle log + 署名付き STH 導入 | witness cosigning, consistency proof, non-inclusion proof, federation |
| Issuer key rotation | 未定義 | epoch + 重複期間 |
| Multi-issuer aggregation | 未対応 | well-known endpoint 標準化 |
| Test vectors | TBD | CFRG style |
| BBS+ Blind issuance spec | draft 段階 | spec 確定追従 |
| Threshold issuance (M-of-N) | 未対応 | threshold BBS+ 研究を追跡 |
| Selective disclosure | ✅ v0.2 で `purchase_tier` / `product_category` 導入 | 追加属性 (membership tier, visit count tier, geographic region etc.) |

---

## 12. References

- IRTF CFRG draft: BBS Signatures — https://datatracker.ietf.org/doc/draft-irtf-cfrg-bbs-signatures/
- W3C VC Data Integrity BBS Cryptosuite — https://www.w3.org/TR/vc-di-bbs/
- Boneh, Boyen, Shacham (2004). "Short Group Signatures"
- Au, Susilo, Mu (2006). "Constant-Size Dynamic k-TAA" (BBS+ origin)
- Camenisch, Drijvers, Lehmann (2016). "Anonymous Attestation Using the Strong Diffie Hellman Assumption Revisited"
- RFC 8032: EdDSA / Ed25519
- RFC 8785: JSON Canonical Serialization (JCS)
- RFC 9380: Hashing to Elliptic Curves
- BLS12-381 curve specification — https://hackmd.io/@benjaminion/bls12-381
- `zkryptium` Rust crate — https://github.com/Cybersecurity-LINKS/zkryptium
- `bls12_381` Rust crate — https://docs.rs/bls12_381
- `ed25519-dalek` Rust crate — https://docs.rs/ed25519-dalek
- `serde_jcs` Rust crate — https://docs.rs/serde_jcs
