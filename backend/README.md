# Jess Book Recommender backend

Cloudflare Worker backend for the book recommendation flow.

## Architecture

- `api`: HTTP routing, CORS, request parsing, and response shaping.
- `application`: `BookRecommenderService` and the AI inference trait.
- `domain`: book candidates, user answers, sessions, and recommendations.
- `infrastructure`: Workers AI and optional Ollama adapters.
- `setup`: environment config and application state wiring.

## Endpoint

`POST /api/v1/books/recommend`

Request body:

```json
{
  "books": [{ "title": "Example", "author": "A. Writer", "average_rating": 4.1, "shelves": ["to-read"] }],
  "answers": [{ "question": "What mood?", "answer": "Cosy and quick." }]
}
```

The response either contains the next question or one final recommendation.
