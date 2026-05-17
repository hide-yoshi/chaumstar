// TypeScript shapes mirroring chaumstar-core's wire types.
// MintState and Credential are passed verbatim through the WASM boundary
// and treated as opaque on the JS side.

export interface PublicKeyset {
	issuer_id: string;
	merchant_id: string;
	keyset_id: string; // 16-char hex
	public_key_bytes: string; // hex of 96 B G2 point
}

export interface MintRequest {
	issuer_id: string;
	merchant_id: string;
	issued_at: string;
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
	issued_at: string;
	timestamp: string;
}

export interface CredentialProof {
	hpk: string; // 64-char hex
	keyset_id: string;
	bbs_proof: string; // hex
}

export interface ReviewPayload {
	version: string;
	review_body: ReviewBody;
	credential_proof: CredentialProof;
	sig: string; // 128-char hex (64 B)
}

// Opaque (round-tripped through WASM).
export type MintState = unknown;
export type Credential = unknown;

export interface MintStartResult {
	state: MintState;
	request: MintRequest;
}
