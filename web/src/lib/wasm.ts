// Thin wrapper around the chaumstar-wasm package. Initializes lazily and
// re-exports the bindings under nicer names.

import init, {
	mintFinish as wasmMintFinish,
	mintStart as wasmMintStart,
	protocolVersion as wasmProtocolVersion,
	publishReview as wasmPublishReview,
	verifyProof as wasmVerifyProof
} from 'chaumstar-wasm';
import type {
	Credential,
	MintRequest,
	MintResponse,
	MintStartResult,
	MintState,
	PublicKeyset,
	ReviewBody,
	ReviewPayload
} from './types';

let ready: Promise<void> | null = null;

export function ensureWasm(): Promise<void> {
	if (!ready) {
		ready = init().then(() => undefined);
	}
	return ready;
}

export async function protocolVersion(): Promise<string> {
	await ensureWasm();
	return wasmProtocolVersion();
}

export async function mintStart(
	keyset: PublicKeyset,
	merchant_id: string,
	issued_at: string
): Promise<MintStartResult> {
	await ensureWasm();
	return wasmMintStart(keyset, merchant_id, issued_at) as MintStartResult;
}

export async function mintFinish(
	state: MintState,
	response: MintResponse
): Promise<Credential> {
	await ensureWasm();
	return wasmMintFinish(state, response) as Credential;
}

export async function publishReview(
	credential: Credential,
	body: ReviewBody
): Promise<ReviewPayload> {
	await ensureWasm();
	return wasmPublishReview(credential, body) as ReviewPayload;
}

export async function verifyProof(
	payload: ReviewPayload,
	keyset: PublicKeyset
): Promise<void> {
	await ensureWasm();
	wasmVerifyProof(payload, keyset);
}
