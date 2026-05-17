// Thin wrapper around the chaumstar-wasm package. Initializes lazily and
// re-exports the bindings under nicer names.

import init, {
	mintFinish as wasmMintFinish,
	mintStart as wasmMintStart,
	protocolVersion as wasmProtocolVersion,
	publishReview as wasmPublishReview,
	verifyInclusion as wasmVerifyInclusion,
	verifyProof as wasmVerifyProof,
	verifySth as wasmVerifySth
} from 'chaumstar-wasm';
import type {
	Credential,
	DisclosureMask,
	InclusionProof,
	MintContext,
	MintResponse,
	MintStartResult,
	MintState,
	PublicKeyset,
	ReviewBody,
	ReviewPayload,
	Sth
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
	ctx: MintContext
): Promise<MintStartResult> {
	await ensureWasm();
	return wasmMintStart(keyset, ctx) as MintStartResult;
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
	body: ReviewBody,
	mask: DisclosureMask
): Promise<ReviewPayload> {
	await ensureWasm();
	return wasmPublishReview(credential, body, mask) as ReviewPayload;
}

export async function verifyProof(
	payload: ReviewPayload,
	keyset: PublicKeyset
): Promise<void> {
	await ensureWasm();
	wasmVerifyProof(payload, keyset);
}

export async function verifySth(sth: Sth, registryPubkeyHex: string): Promise<void> {
	await ensureWasm();
	wasmVerifySth(sth, registryPubkeyHex);
}

export async function verifyInclusion(
	payload: ReviewPayload,
	proof: InclusionProof,
	sth: Sth
): Promise<void> {
	await ensureWasm();
	wasmVerifyInclusion(payload, proof, sth);
}
