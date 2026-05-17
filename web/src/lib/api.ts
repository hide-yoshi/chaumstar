import type {
	MintRequest,
	MintResponse,
	PublicKeyset,
	RegistryKeyResponse,
	ReviewListResponse,
	ReviewPayload,
	ReviewWithProofAndSth,
	Sth
} from './types';

const API = '/api/v1';

async function failOnError(r: Response, op: string): Promise<Response> {
	if (!r.ok) {
		let body = '';
		try {
			body = await r.text();
		} catch {
			/* ignore */
		}
		throw new ApiError(op, r.status, body);
	}
	return r;
}

export class ApiError extends Error {
	constructor(
		public op: string,
		public status: number,
		public body: string
	) {
		super(`${op} failed: ${status} ${body}`);
		this.name = 'ApiError';
	}
}

export async function listKeysets(): Promise<PublicKeyset[]> {
	const r = await fetch(`${API}/keysets`);
	await failOnError(r, 'GET /keysets');
	return r.json();
}

export async function postMint(req: MintRequest): Promise<MintResponse> {
	const r = await fetch(`${API}/mints`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(req)
	});
	await failOnError(r, 'POST /mints');
	return r.json();
}

export async function postReview(payload: ReviewPayload): Promise<ReviewWithProofAndSth> {
	const r = await fetch(`${API}/reviews`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(payload)
	});
	await failOnError(r, 'POST /reviews');
	return r.json();
}

export async function listReviews(): Promise<ReviewListResponse> {
	const r = await fetch(`${API}/reviews`);
	await failOnError(r, 'GET /reviews');
	return r.json();
}

export async function getHealth(): Promise<{ status: string; version: string }> {
	const r = await fetch(`${API}/health`);
	await failOnError(r, 'GET /health');
	return r.json();
}

export async function getRegistryKey(): Promise<RegistryKeyResponse> {
	const r = await fetch(`${API}/registry-key`);
	await failOnError(r, 'GET /registry-key');
	return r.json();
}

export async function getSth(): Promise<Sth> {
	const r = await fetch(`${API}/sth`);
	await failOnError(r, 'GET /sth');
	return r.json();
}
