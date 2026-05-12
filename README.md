# jess-book-recommender

An Astro and Cloudflare Workers app for choosing one next book from a personal library.

Two library sources are supported:

- **Goodreads CSV**: parsed in the browser to find unread books.
- **Fable profile**: paste a `https://fable.co/fabler/<username>` URL, pick one of the profile's book lists, and the backend fetches it from the public Fable API.

In both cases the recommender asks targeted preference questions and displays the final pick. The backend is a small layered Rust Worker that wraps Workers AI or local Ollama generation behind an application service.

## Workspaces

- `frontend`: Astro app: Goodreads CSV parsing, Fable source UI, and source-binding API routes.
- `backend`: Rust Cloudflare Worker exposing:
  - `POST /api/v1/books/recommend`
  - `GET /api/v1/fable/profile/:username`
  - `GET /api/v1/fable/book_lists/:list_id/books`

## Commands

- `npm run dev`: run frontend and backend dev servers.
- `npm run build:frontend`: check and build the Astro app.
- `npm run build:backend`: build the Worker with Wrangler.
- `npm run verify`: build the frontend and run `cargo check` for the backend.
