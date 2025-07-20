import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ request }) => {
	
    const headers = new Headers(request.headers);

	return {
		headers: JSON.parse(JSON.stringify(Object.fromEntries(headers))),
	};
};