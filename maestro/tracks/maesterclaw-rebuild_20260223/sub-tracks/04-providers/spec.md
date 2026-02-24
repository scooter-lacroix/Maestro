# Subtrack 04: Provider Implementations

## Objective
Implement the Provider trait and concrete implementations for OpenAI, Anthropic, Ollama, and OpenRouter.

## Requirements

### R1: Enhanced Provider Trait
- name() returns provider identifier
- capabilities() returns ProviderCapabilities (streaming, tools, vision)
- chat(request: ChatRequest) -> ChatResponse (async)
- stream_chat(request: ChatRequest) -> BoxStream<StreamChunk> (async)
- chat_with_tools(messages, tools) -> ChatResponse (async)
- supports_native_tools() -> bool
- warmup() -> Result<()> (async)
- health_check() -> bool (async)

### R2: OpenAI Provider
- GPT-4/GPT-4-turbo support
- Native tool calling
- Streaming responses
- Error handling (rate limits, auth, context length)

### R3: Anthropic Provider
- Claude 3.5 Sonnet support
- Native tool use
- Streaming responses
- Error handling

### R4: Ollama Provider
- Local model support (llama3, mistral, etc.)
- Tool calling (where supported)
- Streaming responses

### R5: OpenRouter Provider
- Multi-provider routing
- Model selection
- Cost tracking

## Acceptance Criteria
- [ ] Enhanced Provider trait defined
- [ ] OpenAI provider working with real API
- [ ] Anthropic provider working with real API
- [ ] Ollama provider working with local models
- [ ] OpenRouter provider working
- [ ] Tool calling verified for OpenAI and Anthropic
- [ ] >98% test coverage
