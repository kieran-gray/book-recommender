export const siteConfig = {
	url: 'https://kgdev.me',
	brand: {
		name: 'Jess Book',
		tld: 'Recommender',
		accentDot: true,
		favicon:
			'data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text y=%22.9em%22 font-size=%2290%22>📚</text></svg>'
	},
	meta: {
		title: 'jess-book-recommender',
		description:
			'Upload a Goodreads library export, answer a few targeted questions, and choose the next unread book.',
		locale: 'en'
	}
} as const;
