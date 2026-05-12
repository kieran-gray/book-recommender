# Jess Book Recommender frontend

Astro app for uploading a Goodreads CSV export and working through the book recommendation flow.

The app reads the CSV locally in the browser. Only the derived unread book list and question answers are sent to `${PUBLIC_BACKEND_URL}/api/v1/books/recommend`.
