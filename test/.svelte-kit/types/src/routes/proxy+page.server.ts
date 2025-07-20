// @ts-nocheck
import type { PageServerLoad } from './$types';

export const load = async ({ request }: Parameters<PageServerLoad>[0]) => {
	
    const headers = new Headers(request.headers);

	return {
		headers: JSON.parse(JSON.stringify(Object.fromEntries(headers))),
	};
};