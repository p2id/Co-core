/**
 * Wire format for remote agents. Any process — in any language — that
 * accepts this request shape over HTTP and returns this response shape
 * can plug into the orchestrator through {@link RemoteAgent}. This is
 * what keeps the core language-agnostic: orchestration lives here,
 * execution can live in Python, Go, Rust, or anything else.
 */
export interface RemoteAgentRequest {
  taskId: string;
  capability: string;
  payload: unknown;
  threadId: string;
}

export interface RemoteAgentResponse {
  success: boolean;
  output?: unknown;
  error?: string;
}
