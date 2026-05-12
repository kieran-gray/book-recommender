export {};

interface BookCandidate {
	title: string;
	author: string;
	average_rating: number | null;
	shelves: string[];
}

interface Answer {
	question: string;
	answer: string;
}

interface Recommendation {
	title: string;
	author: string;
	reason: string;
}

interface ApiResponse {
	done: boolean;
	questions: string[];
	recommendations: Recommendation[];
}

interface FableList {
	id: string;
	name: string;
	count: number;
}

interface FableProfile {
	user_id: string;
	display_name: string;
	lists: FableList[];
}

interface FableBooksResponse {
	books: BookCandidate[];
}

type Source = 'csv' | 'fable';

function parseCsv(text: string): Record<string, string>[] {
	const rows: string[][] = [];
	let row: string[] = [];
	let cell = '';
	let quoted = false;

	for (let i = 0; i < text.length; i += 1) {
		const char = text[i];
		const next = text[i + 1];

		if (char === '"' && quoted && next === '"') {
			cell += '"';
			i += 1;
		} else if (char === '"') {
			quoted = !quoted;
		} else if (char === ',' && !quoted) {
			row.push(cell);
			cell = '';
		} else if ((char === '\n' || char === '\r') && !quoted) {
			if (char === '\r' && next === '\n') i += 1;
			row.push(cell);
			if (row.some((value) => value.trim().length > 0)) rows.push(row);
			row = [];
			cell = '';
		} else {
			cell += char;
		}
	}

	row.push(cell);
	if (row.some((value) => value.trim().length > 0)) rows.push(row);
	if (rows.length < 2) return [];

	const headers = rows[0]!.map((header) => header.trim());
	return rows.slice(1).map((values) => {
		const record: Record<string, string> = {};
		headers.forEach((header, index) => {
			record[header] = values[index]?.trim() ?? '';
		});
		return record;
	});
}

function isUnread(row: Record<string, string>): boolean {
	const shelf = row['Exclusive Shelf']?.toLowerCase();
	const dateRead = row['Date Read'];
	const readCount = Number.parseInt(row['Read Count'] ?? '0', 10);
	return shelf === 'to-read' || (!dateRead && (!Number.isFinite(readCount) || readCount === 0));
}

function toBooks(rows: Record<string, string>[]): BookCandidate[] {
	const seen = new Set<string>();
	return rows
		.filter(isUnread)
		.map((row) => {
			const title = row.Title || row['Book Title'] || '';
			const author = row.Author || row['Author l-f'] || 'Unknown author';
			const key = `${title.toLowerCase()}::${author.toLowerCase()}`;
			const shelves = [row.Bookshelves, row['Exclusive Shelf']]
				.flatMap((value) => (value ?? '').split(','))
				.map((value) => value.trim())
				.filter(Boolean);
			const rating = Number.parseFloat(row['Average Rating'] ?? '');
			return {
				title,
				author,
				average_rating: Number.isFinite(rating) ? rating : null,
				shelves: Array.from(new Set(shelves)),
				key
			};
		})
		.filter((book) => {
			if (!book.title || seen.has(book.key)) return false;
			seen.add(book.key);
			return true;
		})
		.map(({ key: _key, ...book }) => book);
}

function parseFableUsername(input: string): string | null {
	const trimmed = input.trim();
	if (!trimmed) return null;

	const slugMatch = /^[A-Za-z0-9_-]+$/;
	if (slugMatch.test(trimmed)) return trimmed;

	try {
		const url = new URL(trimmed);
		if (!/(^|\.)fable\.co$/i.test(url.hostname)) return null;
		const parts = url.pathname.split('/').filter(Boolean);
		const fablerIndex = parts.findIndex((part) => part.toLowerCase() === 'fabler');
		const candidate = fablerIndex >= 0 ? parts[fablerIndex + 1] : parts[parts.length - 1];
		if (candidate && slugMatch.test(candidate)) return candidate;
		return null;
	} catch {
		return null;
	}
}

function renderStyledText(target: HTMLElement, text: string) {
	target.replaceChildren();
	const parts = text.split(/(\*\*[^*]+\*\*)/g);
	for (const part of parts) {
		if (!part) continue;
		if (part.startsWith('**') && part.endsWith('**') && part.length > 4) {
			const strong = document.createElement('strong');
			strong.textContent = part.slice(2, -2);
			target.appendChild(strong);
		} else {
			target.appendChild(document.createTextNode(part));
		}
	}
}

function setup() {
	const root = document.getElementById('book-recommender');
	if (!root) return;
	if (root.dataset.ready === 'true') return;
	root.dataset.ready = 'true';
	const app = root;

	const endpoint = root.dataset.endpoint ?? '/api/recommend';
	const fileInput = document.getElementById('goodreads-csv') as HTMLInputElement | null;
	const summary = document.getElementById('library-summary');
	const status = document.getElementById('book-status');
	const form = document.getElementById('question-form') as HTMLFormElement | null;
	const questionLabel = document.getElementById('question-label');
	const progress = document.getElementById('question-progress');
	const answerInput = document.getElementById('answer-input') as HTMLTextAreaElement | null;
	const submit = document.getElementById('submit-answer') as HTMLButtonElement | null;
	const skip = document.getElementById('skip-question') as HTMLButtonElement | null;
	const recPanel = document.getElementById('recommendation-panel');
	const recTitle = document.getElementById('recommendation-title');
	const recAuthor = document.getElementById('recommendation-author');
	const recReason = document.getElementById('recommendation-reason');
	const recProgress = document.getElementById('recommendation-progress');
	const nextRecommendation = document.getElementById('next-recommendation') as HTMLButtonElement | null;
	const startOver = document.getElementById('start-over');
	const sourceTabs = Array.from(
		document.querySelectorAll<HTMLButtonElement>('.source-tab')
	);
	const sourcePanels = Array.from(
		document.querySelectorAll<HTMLElement>('.source-panel')
	);
	const fableForm = document.getElementById('fable-form') as HTMLFormElement | null;
	const fableUrl = document.getElementById('fable-url') as HTMLInputElement | null;
	const fableLists = document.getElementById('fable-lists');
	const fableListOptions = document.getElementById('fable-list-options');

	if (
		!fileInput ||
		!summary ||
		!status ||
		!form ||
		!questionLabel ||
		!progress ||
		!answerInput ||
		!submit ||
		!skip ||
		!recPanel ||
		!recTitle ||
		!recAuthor ||
		!recReason ||
		!recProgress ||
		!nextRecommendation ||
		!startOver ||
		!fableForm ||
		!fableUrl ||
		!fableLists ||
		!fableListOptions
	) {
		return;
	}
	const els = {
		fileInput,
		summary,
		status,
		form,
		questionLabel,
		progress,
		answerInput,
		submit,
		skip,
		recPanel,
		recTitle,
		recAuthor,
		recReason,
		recProgress,
		nextRecommendation,
		startOver,
		fableForm,
		fableUrl,
		fableLists,
		fableListOptions
	};

	let books: BookCandidate[] = [];
	let answers: Answer[] = [];
	let questions: string[] = [];
	let questionIndex = 0;
	let recommendations: Recommendation[] = [];
	let recommendationIndex = 0;
	let currentSource: Source = 'csv';

	function setStatus(message: string) {
		els.status.textContent = message;
		app.dataset.status = message ? 'visible' : 'empty';
	}

	function setBusy(busy: boolean) {
		app.dataset.state = busy ? 'busy' : 'ready';
		els.submit.disabled = busy;
		els.skip.disabled = busy;
		els.fileInput.disabled = busy;
		const fableSubmit = els.fableForm.querySelector<HTMLButtonElement>('button[type="submit"]');
		if (fableSubmit) fableSubmit.disabled = busy;
		els.fableUrl.disabled = busy;
		els.fableListOptions
			.querySelectorAll<HTMLButtonElement>('button')
			.forEach((btn) => {
				btn.disabled = busy;
			});
	}

	function selectSource(source: Source) {
		if (currentSource === source) return;
		currentSource = source;
		sourceTabs.forEach((tab) => {
			tab.setAttribute(
				'aria-selected',
				tab.dataset.source === source ? 'true' : 'false'
			);
		});
		sourcePanels.forEach((panel) => {
			panel.hidden = panel.dataset.sourcePanel !== source;
		});
		books = [];
		answers = [];
		questions = [];
		questionIndex = 0;
		recommendations = [];
		recommendationIndex = 0;
		els.form.hidden = true;
		els.recPanel.hidden = true;
		els.summary.hidden = true;
		els.fableLists.hidden = true;
		els.fableListOptions.replaceChildren();
		app.dataset.state = 'ready';
		setStatus('');
	}

	sourceTabs.forEach((tab) => {
		tab.addEventListener('click', () => {
			const source = tab.dataset.source as Source | undefined;
			if (source) selectSource(source);
		});
	});

	function showRecommendation() {
		const current = recommendations[recommendationIndex];
		if (!current) return;
		els.form.hidden = true;
		els.recPanel.hidden = false;
		renderStyledText(els.recTitle, current.title);
		renderStyledText(els.recAuthor, `by ${current.author}`);
		renderStyledText(els.recReason, current.reason);
		els.recProgress.textContent = `${recommendationIndex + 1} of ${recommendations.length}`;
		const hasNext = recommendationIndex < recommendations.length - 1;
		els.nextRecommendation.hidden = !hasNext;
		els.nextRecommendation.disabled = !hasNext;
	}

	function showQuestion() {
		const currentQuestion = questions[questionIndex];
		if (!currentQuestion) {
			void askBackend();
			return;
		}

		els.form.hidden = false;
		els.recPanel.hidden = true;
		renderStyledText(els.questionLabel, currentQuestion);
		els.progress.textContent = `${questionIndex + 1} / ${questions.length}`;
		els.answerInput.value = '';
		els.answerInput.focus();
		setStatus('');
	}

	async function askBackend() {
		setBusy(true);
		setStatus(answers.length === 0 ? 'Generating questions...' : 'Choosing a book...');
		try {
			const response = await fetch(endpoint, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ books, answers })
			});

			if (!response.ok) {
				const payload = (await response.json().catch(() => null)) as { error?: string } | null;
				throw new Error(payload?.error ?? `Request failed with ${response.status}`);
			}

			const payload = (await response.json()) as ApiResponse;
			if (payload.done && payload.recommendations.length > 0) {
				recommendations = payload.recommendations;
				recommendationIndex = 0;
				showRecommendation();
				setStatus('');
			} else if (payload.questions.length > 0) {
				questions = payload.questions;
				questionIndex = 0;
				showQuestion();
			} else {
				throw new Error('The recommender returned an empty response.');
			}
		} catch (error) {
			setStatus(error instanceof Error ? error.message : 'Something went wrong.');
		} finally {
			setBusy(false);
		}
	}

	async function loadFableProfile(rawInput: string) {
		const username = parseFableUsername(rawInput);
		if (!username) {
			setStatus('Enter a Fable profile URL or username.');
			return;
		}

		setBusy(true);
		setStatus('Looking up profile...');
		els.fableLists.hidden = true;
		els.fableListOptions.replaceChildren();
		try {
			const response = await fetch(`/api/fable/profile/${encodeURIComponent(username)}`);
			if (!response.ok) {
				const payload = (await response.json().catch(() => null)) as { error?: string } | null;
				throw new Error(payload?.error ?? `Could not load profile (${response.status})`);
			}
			const profile = (await response.json()) as FableProfile;
			renderFableLists(profile);
			setStatus(`Loaded ${profile.lists.length} lists for ${profile.display_name}.`);
		} catch (error) {
			setStatus(error instanceof Error ? error.message : 'Could not load Fable profile.');
		} finally {
			setBusy(false);
		}
	}

	function renderFableLists(profile: FableProfile) {
		els.fableListOptions.replaceChildren();
		if (profile.lists.length === 0) {
			els.fableLists.hidden = true;
			return;
		}

		for (const list of profile.lists) {
			const li = document.createElement('li');
			const button = document.createElement('button');
			button.type = 'button';
			button.className = 'fable-list-option';
			const nameLabel = document.createElement('span');
			nameLabel.className = 'list-name';
			nameLabel.textContent = list.name;
			const countBadge = document.createElement('span');
			countBadge.className = 'list-count';
			countBadge.textContent = `${list.count} ${list.count === 1 ? 'book' : 'books'}`;
			button.appendChild(nameLabel);
			button.appendChild(countBadge);
			button.addEventListener('click', () => {
				void loadFableList(list);
			});
			li.appendChild(button);
			els.fableListOptions.appendChild(li);
		}
		els.fableLists.hidden = false;
	}

	async function loadFableList(list: FableList) {
		setBusy(true);
		setStatus(`Loading "${list.name}"...`);
		try {
			const response = await fetch(
				`/api/fable/book_lists/${encodeURIComponent(list.id)}/books`
			);
			if (!response.ok) {
				const payload = (await response.json().catch(() => null)) as { error?: string } | null;
				throw new Error(payload?.error ?? `Could not load list (${response.status})`);
			}
			const payload = (await response.json()) as FableBooksResponse;
			books = payload.books ?? [];
			answers = [];
			questions = [];
			questionIndex = 0;
			els.summary.hidden = false;
			els.summary.textContent = `Loaded ${books.length} books from "${list.name}".`;
			if (books.length === 0) {
				setStatus('That Fable list is empty.');
				return;
			}
			await askBackend();
		} catch (error) {
			setStatus(error instanceof Error ? error.message : 'Could not load that Fable list.');
		} finally {
			setBusy(false);
		}
	}

	fileInput.addEventListener('change', async () => {
		const file = fileInput.files?.[0];
		if (!file) return;
		setStatus('Reading CSV...');
		const text = await file.text();
		const rows = parseCsv(text);
		books = toBooks(rows);
		answers = [];
		questions = [];
		questionIndex = 0;
		app.dataset.state = 'uploaded';
		summary.hidden = false;
		summary.textContent = `Found ${books.length} unread books from ${rows.length} Goodreads rows.`;
		if (books.length === 0) {
			setStatus('No unread books were found in this export.');
			return;
		}
		await askBackend();
	});

	fableForm.addEventListener('submit', async (event) => {
		event.preventDefault();
		await loadFableProfile(fableUrl.value);
	});

	form.addEventListener('submit', async (event) => {
		event.preventDefault();
		const answer = answerInput.value.trim();
		if (!answer) return;
		answers.push({ question: questions[questionIndex] ?? '', answer });
		questionIndex += 1;
		showQuestion();
	});

	skip.addEventListener('click', async () => {
		answers.push({ question: questions[questionIndex] ?? '', answer: 'No preference.' });
		questionIndex += 1;
		showQuestion();
	});

	nextRecommendation.addEventListener('click', () => {
		if (recommendationIndex < recommendations.length - 1) {
			recommendationIndex += 1;
			showRecommendation();
		}
	});

	startOver.addEventListener('click', () => {
		answers = [];
		questions = [];
		questionIndex = 0;
		recommendations = [];
		recommendationIndex = 0;
		recPanel.hidden = true;
		if (books.length > 0) void askBackend();
	});
}

document.addEventListener('astro:page-load', setup);
setup();
