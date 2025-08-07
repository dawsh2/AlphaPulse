# AlphaPulse AI Agent Implementation Plan

## 1. Core Concept: Retrieval-Augmented Generation (RAG)

To enable an LLM to reason about specific documents like academic papers, we will implement a **Retrieval-Augmented Generation (RAG)** pipeline. This is the industry-standard approach for building sophisticated Q&A and document analysis tools.

**How RAG Works:**

1.  **Indexing (Offline):**
    -   **Chunking:** Papers are broken down into small, manageable text chunks.
    -   **Embedding:** Each chunk is converted into a numerical vector (an "embedding") that captures its semantic meaning.
    -   **Storing:** These embeddings are stored in a specialized **vector database**.
2.  **Retrieval & Generation (Real-time):**
    -   **User Query:** A user asks a question.
    -   **Retrieve:** The system searches the vector database for the most semantically relevant text chunks based on the user's query.
    -   **Augment & Generate:** The user's question and the retrieved text chunks are combined into a detailed prompt, which is then sent to an LLM to generate a context-aware answer.

This approach is highly scalable and avoids the context window limitations of LLMs, allowing the system to reason across a vast library of documents.

---

## 2. Phased Implementation Plan

### Phase 1: The MVP - "Summarize this Paper"

This is a low-complexity, high-impact feature to provide immediate value and validate the basic LLM integration.

1.  **Backend Endpoint:**
    -   **Route:** `POST /api/articles/{article_id}/summarize`
    -   **Logic:**
        1.  Retrieve the article's abstract from the primary database.
        2.  Send the abstract to a general-purpose LLM API (e.g., Gemini API).
        3.  Use a specific, engineered prompt: `"Summarize the following academic abstract for a quantitative finance professional. Focus on the key methodology, findings, and potential applications."`
        4.  Stream the LLM's response back to the frontend.

2.  **Frontend UI:**
    -   Add a "Summarize with AI" button to the article view.
    -   On click, call the `/summarize` endpoint.
    -   Display a loading indicator while waiting for the response.
    -   Render the streamed summary in a modal or an expandable section.

**Goal:** Deliver a "magical" AI feature quickly and establish the API communication pattern.

### Phase 2: The Foundation - Building the RAG Pipeline

This is the core backend infrastructure for all future advanced AI features.

1.  **Select and Integrate a Vector Database:**
    -   **Local/Development:** Start with `ChromaDB` or `FAISS` for simplicity.
    -   **Production:** Plan to migrate to a managed service like `Pinecone`, `Weaviate`, or a cloud provider's solution (e.g., Vertex AI Vector Search).

2.  **Enhance the Data Ingestion Worker:**
    -   Modify the existing background worker that fetches papers from arXiv.
    -   **New Steps:** After an article is saved to the main database, the worker must also:
        1.  **Chunk Text:** Break the article's text (starting with the abstract) into small, overlapping chunks (e.g., 500 characters). Use a library like `LangChain` to handle this reliably.
        2.  **Generate Embeddings:** For each chunk, call an embedding model API (e.g., `text-embedding-004`) to create its vector.
        3.  **Store in Vector DB:** Insert the text chunk, its embedding, and the source `article_id` into the vector database.

3.  **Implement the Core Q&A Endpoint:**
    -   **Route:** `POST /api/chat`
    -   **Logic:**
        1.  Receive a user's question in the request body.
        2.  Generate an embedding for the user's question using the same embedding model.
        3.  Query the vector database to find the top `k` (e.g., 3-5) most relevant text chunks.
        4.  Construct an augmented prompt for the LLM, including the user's question and the retrieved chunks as context.
        5.  Send the prompt to the LLM and stream the response back to the client.

**Goal:** Build the foundational, scalable infrastructure for all knowledge-based AI features.

### Phase 3: The Payoff - Advanced, Interactive Features

With the RAG pipeline in place, we can build the high-value features.

1.  **Cross-Paper Interactive Chat:**
    -   **UI:** A dedicated chat interface in the application.
    -   **Functionality:** Users can ask questions that span the entire library of indexed papers. The backend uses the `/api/chat` endpoint.
    -   **Enhancement:** The LLM response should include citations, linking back to the source articles from which the information was retrieved.

2.  **Custom Code & Config Generation:**
    -   **User Prompt Example:** `"Using the methodology from paper X, write a Python function using the `pandas` library to calculate the momentum signal described."`
    -   **Backend Logic:**
        1.  The RAG pipeline retrieves the specific, relevant sections of the paper describing the methodology.
        2.  A highly-engineered prompt is constructed, instructing the LLM to act as an expert programmer and generate code based *only* on the provided context.
        3.  The generated code is returned to the user, potentially in a code editor component with syntax highlighting.

---

## 3. Key Tips & Best Practices

-   **Start with Abstracts:** Begin by indexing only the abstracts. This is significantly simpler and more cost-effective than processing full PDFs, which requires a separate parsing step (e.g., with `PyMuPDF`).
-   **Cost Management:** Be aware that embedding generation is a one-time cost per document chunk, while LLM calls are per-query. Monitor API usage closely.
-   **Streaming for UX:** LLM responses can be slow. Streaming responses token-by-token is **essential** for a good user experience, as it shows the user that the system is working.
-   **Model Selection:** Use a tiered approach to models. A fast, inexpensive model can be used for simple summarization, while a more powerful model (e.g., Gemini 1.5 Pro) should be reserved for complex Q&A and code generation.
-   **Prompt Engineering:** The success of this entire system hinges on the quality of the prompts. They must be clear, specific, and instruct the model on how to use the provided context. This will require significant iteration and refinement.
