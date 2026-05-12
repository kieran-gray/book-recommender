import type { APIRoute } from 'astro';

export const prerender = false;

export const POST: APIRoute = async ({ request, locals }) => {
	const env = locals.runtime.env;
	return env.BACKEND.fetch('https://backend/api/v1/books/recommend', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: await request.text()
	});
};
