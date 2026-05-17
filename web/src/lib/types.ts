// TypeScript shapes mirroring chaumstar-core's wire types.
// MintState and Credential are passed verbatim through the WASM boundary;
// we keep them statically typed for safer composer / wallet code.

export type PurchaseTier = 'low' | 'mid' | 'high';
export type ProductCategory = 'drinks' | 'food' | 'merch';

export interface MintContext {
	merchant_id: string;
	purchase_tier: PurchaseTier;
	product_category: ProductCategory;
}

export interface DisclosureMask {
	disclose_tier: boolean;
	disclose_category: boolean;
}

export const NO_DISCLOSURE: DisclosureMask = {
	disclose_tier: false,
	disclose_category: false
};

export interface PublicKeyset {
	issuer_id: string;
	merchant_id: string;
	keyset_id: string; // 16-char hex
	public_key_bytes: string; // hex of 96 B G2 point
}

export interface MintRequest {
	issuer_id: string;
	merchant_id: string;
	purchase_tier: PurchaseTier;
	product_category: ProductCategory;
	keyset_id: string;
	commitment_bytes: string; // hex
}

export interface MintResponse {
	blind_signature_bytes: string; // hex of 80 B
}

export interface ReviewBody {
	text: string;
	rating: number;
	merchant_id: string;
	issuer_id: string;
	timestamp: string;
}

export interface CredentialProof {
	hpk: string; // 64-char hex
	keyset_id: string;
	bbs_proof: string; // hex
	purchase_tier: PurchaseTier | null;
	product_category: ProductCategory | null;
}

export interface ReviewPayload {
	version: string;
	review_body: ReviewBody;
	credential_proof: CredentialProof;
	sig: string; // 128-char hex (64 B)
}

export interface Sth {
	tree_size: number;
	root_hash: string; // hex 32B
	timestamp: string;
	sig: string; // hex 64B
}

export interface InclusionProof {
	leaf_index: number;
	tree_size: number;
	path: string[]; // hex hashes
}

export interface ReviewWithProof {
	payload: ReviewPayload;
	inclusion_proof: InclusionProof;
}

export interface ReviewListResponse {
	reviews: ReviewWithProof[];
	sth: Sth;
}

export interface ReviewWithProofAndSth {
	payload: ReviewPayload;
	inclusion_proof: InclusionProof;
	sth: Sth;
}

export interface RegistryKeyResponse {
	public_key: string; // hex 32B
}

export interface Credential {
	version: string;
	hpk: string;
	hsk: string;
	blind_factor: string;
	blind_signature: string;
	keyset: PublicKeyset;
	merchant_id: string;
	purchase_tier: PurchaseTier;
	product_category: ProductCategory;
}

// Opaque (round-tripped through WASM).
export type MintState = unknown;

export interface MintStartResult {
	state: MintState;
	request: MintRequest;
}

/** Convert ¥ amount → canonical tier (matches Rust PurchaseTier::from_yen). */
export function tierFromYen(amount: number): PurchaseTier {
	if (amount < 1_000) return 'low';
	if (amount < 5_000) return 'mid';
	return 'high';
}
