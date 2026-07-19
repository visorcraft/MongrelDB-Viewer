# Agent (chat)

**Agent** is an OpenAI-compatible chat surface that can call the same tools as
the MCP bridge against the **currently open** database.

## Configuration

| Field | Purpose |
| ----- | ------- |
| Base URL | OpenAI-compatible API root (placeholder examples: `https://api.openai.com/v1` or a local Ollama `/v1` endpoint) |
| API key | Bearer token for that API - leave empty only if the endpoint allows it |
| Model | Model id your endpoint expects |
| System prompt | Optional override |

Keys stay in process memory for the session. Do not commit keys or paste them
into public issues.

## Using chat

1. Connect a database first (Direct or Server).  
2. Configure the endpoint and model.  
3. Type a question (e.g. list tables, describe indexes, draft SQL).  
4. Send - the agent may call tools such as `list_tables`, `execute_sql`,
   `semantic_search`, etc.  

Probe can check connectivity without a full agent loop when available.

## Behavior notes

- Tools operate on the open connection - disconnecting clears the working set.  
- Semantic search via tools is still **per table**, same as the ANN page.  
- The system prompt steers the model toward honest MongrelDB capabilities (no
  invented vectors).  

## Safety

- Prefer read-only SQL exploration until you trust the model and endpoint.  
- Treat remote model providers as third parties for any data you send in prompts
  or tool results.  

Related: [MCP](mcp.md) · [SQL](sql.md) · [Security policy](../SECURITY.md)
