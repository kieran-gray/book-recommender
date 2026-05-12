import type { APIRoute } from 'astro';

export const prerender = false;

export const GET: APIRoute = async ({ params, locals }) => {
	const env = locals.runtime.env;
	const username = encodeURIComponent(params.username ?? '');
	return env.BACKEND.fetch(`https://backend/api/v1/fable/profile/${username}`, {
		method: 'GET',
		headers: { Accept: 'application/json' }
	});
};
