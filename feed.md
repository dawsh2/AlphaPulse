# AlphaPulse Feed Implementation Plan

## 1. Executive Summary

This document outlines the strategy for implementing a functional, reliable, and high-performance news feed for the AlphaPulse web application. The feed will be populated with the latest quantitative finance papers from arXiv.

This plan moves beyond simple web scraping to a robust architecture using the official arXiv API, a background worker for data fetching, and an enhanced database schema. It prioritizes reliability, performance, and respectful use of the arXiv service.

## 2. Core Requirements

- **Data Source:** Fetch papers from the `q-fin` (Quantitative Finance) section of arXiv.
- **Backend Storage:** Persist articles in a local database to ensure fast API responses and minimize load on the arXiv servers.
- **User Interface:** Display articles in a paginated list, similar to Hacker News, not infinite scroll.
- **Data Integrity:** Ensure articles are not duplicated and are stored with accurate metadata.
- **Robustness:** The data fetching process must be resilient to network errors and API downtime.

## 3. Architecture & Design

### 3.1. Backend (`pulse-engine`)

#### Data Source: arXiv API
Instead of scraping HTML, we will use the official **arXiv API**. This provides structured, reliable data and is the recommended method for programmatic access.

- **Dependency:** The `arxiv` Python library will be added to `requirements.txt` to serve as a clean wrapper for the API.

#### Database Model: Enhanced for Performance and Querying
The database schema will be updated to properly store article and author data, optimized for querying.

- **File:** `pulse-engine/models.py`
- **Key Improvements:**
    - A separate `Author` table with a many-to-many relationship to `Article`.
    - An index on the `submitted_date` column for fast sorting.
    - A dedicated `url` field for the direct link to the arXiv paper.
    - An `updated_at` timestamp to track record changes.

```python
# Association table for the many-to-many relationship
article_authors = db.Table('article_authors',
    db.Column('article_id', db.String(255), db.ForeignKey('articles.id'), primary_key=True),
    db.Column('author_id', db.Integer, db.ForeignKey('authors.id'), primary_key=True)
)

class Article(db.Model):
    __tablename__ = 'articles'
    id = db.Column(db.String(255), primary_key=True) # arXiv ID (e.g., '2308.00123v1')
    url = db.Column(db.String, nullable=False)
    title = db.Column(db.String, nullable=False)
    abstract = db.Column(db.Text, nullable=False)
    submitted_date = db.Column(db.DateTime, nullable=False, index=True) # Indexed for performance
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    authors = db.relationship('Author', secondary=article_authors, lazy='subquery',
        backref=db.backref('articles', lazy=True))

class Author(db.Model):
    __tablename__ = 'authors'
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String, unique=True, nullable=False)
```

#### Data Fetching: Asynchronous Background Worker
To ensure the system is robust and scalable, data fetching will be handled by a background worker process using a task queue like **Celery** or **RQ**.

- **Benefits:**
    - **Reliability:** The process can be configured to automatically retry on failure with exponential backoff.
    - **Scheduling:** The task can be scheduled to run periodically (e.g., every hour) via a cron job or Celery Beat.
    - **Performance:** The main web server remains fast and responsive, as it is not blocked by the data fetching task.
- **Logic:** The worker task will:
    1.  Fetch the latest `q-fin` papers using the `arxiv` library.
    2.  For each paper, check if its unique arXiv ID already exists in the `Article` table.
    3.  If the article is new, create `Author` records for any new authors.
    4.  Create the `Article` record and establish its relationships with the authors.
    5.  Implement `time.sleep(1)` between API calls to respect arXiv's rate limits.
    6.  Include a descriptive `User-Agent` in all requests.

#### API Endpoint: Paginated Feed
The API will provide a paginated endpoint for the frontend.

- **Route:** `GET /api/feed`
- **Query Parameters:**
    - `page` (integer, default: 1)
    - `search` (string, optional): Filters by keyword in title/abstract.
- **Response Structure:**
```json
{
  "status": "success",
  "data": [
    {
      "id": "2308.00123v1",
      "url": "http://arxiv.org/abs/2308.00123v1",
      "title": "A Paper on Quantitative Finance",
      "abstract": "...",
      "authors": ["Author One", "Author Two"],
      "submitted_date": "2023-08-01T18:00:00Z"
    }
  ],
  "pagination": {
    "current_page": 1,
    "next_page": 2,
    "prev_page": null,
    "total_pages": 50,
    "total_items": 750
  }
}
```

### 3.2. Frontend (`alphapulse-react`)

#### UI/UX: Pagination and State Handling
The frontend will provide a clean, paginated user experience.

- **Pagination:** Instead of infinite scroll, the UI will feature "Next" and "Previous" buttons, driven by the `pagination` object from the API.
- **State Management:** The `HomePage.tsx` component will manage the following states:
    - `isLoading`: When true, **skeleton loaders** will be displayed to indicate content is being fetched.
    - `error`: If not null, a user-friendly error message will be shown.
    - `data`: If it's an empty array, a "No articles found" message will be displayed.

---

## 4. Implementation Plan

### Phase 1: Backend Setup (1-2 days)
1.  **Update Dependencies:** Add `arxiv` and `celery` (or `rq`) to `requirements.txt`.
2.  **Update Database Models:** Implement the revised `Article` and `Author` models in `models.py`.
3.  **Database Migration:** Generate and apply the database migration to create the new tables.

### Phase 2: Backend Logic (2-3 days)
1.  **Create Worker Task:** Implement the data fetching and storage logic in a new `tasks.py` file.
2.  **Implement CLI Command:** Create a Flask CLI command (`flask fetch-feed`) to manually trigger the worker for testing.
3.  **Schedule the Task:** Configure a scheduler (e.g., Celery Beat) to run the task periodically.
4.  **Create API Endpoint:** Implement the `GET /api/feed` endpoint in `app.py`.

### Phase 3: Frontend Integration (2-3 days)
1.  **Update API Service:** Add a `getFeed(page, search)` function to `src/api/apiService.ts`.
2.  **Build `HomePage.tsx`:**
    - Implement state management for data, loading, error, and current page.
    - Fetch data using the `getFeed` service.
    - Render the list of articles.
    - Render pagination controls based on the API response.
    - Implement skeleton loading and error/empty states.
3.  **Add Search Input:** Add a search bar to the UI that triggers API calls with the `search` query parameter.

## 5. Best Practices Checklist

This plan explicitly incorporates the following best practices:

-   [x] **Use Official API:** Prioritize the official arXiv API over web scraping.
-   [x] **Respectful API Usage:** Implement rate limiting (`time.sleep`) and a `User-Agent`.
-   [x] **Data Deduplication:** Use the unique arXiv ID as the primary key to prevent duplicates.
-   [x] **Robust Error Handling:** The background worker will handle network and parsing errors gracefully.
-   [x] **Asynchronous Operations:** Use a task queue to prevent blocking the main application.
-   [x] **Clean API Design:** Provide a paginated and filterable API endpoint.
-   [x] **Modern Frontend UX:** Implement skeleton loaders and clear state handling (loading, error, empty).
-   [x] **Secure by Default:** Sanitize any data rendered in the HTML to prevent XSS, although the risk is low with a trusted source like arXiv.
