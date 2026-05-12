import type { APIRoute } from 'astro';

export const prerender = false;

export const GET: APIRoute = async ({ params, locals }) => {
	const env = locals.runtime.env;
	const id = encodeURIComponent(params.id ?? '');
	return env.BACKEND.fetch(`https://backend/api/v1/fable/book_lists/${id}/books`, {
		method: 'GET',
		headers: { Accept: 'application/json' }
	});
};
