# Maestro - Product Guidelines

## Content Tone & Style

Maestro communicates with an **approachable and friendly** tone. Documentation and user-facing content should be conversational with explanations suitable for beginners, avoiding overly technical jargon unless necessary.

## Information Presentation

Content should be presented with the following priority:

1. **Step-by-Step Instructions + Visual Examples**: Clear numbered lists paired with code snippets and examples
2. **Progressive Disclosure**: Introduce simple concepts first, then reveal advanced options
3. **Real-World Scenarios**: Include usage examples that demonstrate practical applications

## Error Handling & User Support

Maestro prioritizes a supportive approach to errors:

1. **Clear Error Messages**: Actionable guidance on what went wrong and next steps
2. **Recovery Options**: Offer rollback, fix, or retry options when possible
3. **Learning Explanations**: Help users understand the root cause to prevent future issues
4. **Preventative Guidance**: Suggest measures to avoid common mistakes

## Visual Style

Documentation and command output should be:

1. **Clean & Uncluttered**: Clear section headers with ample whitespace
2. **Terminal Color Hints**: Green for success, red for errors, yellow for warnings
3. **Consistent Formatting**: Use markdown tables, code blocks, and lists uniformly
4. **Tasteful Emojis**: Extremely sparing use - only for key emphasis, never decorative

## Workflow Guidance

For complex, multi-step operations:

1. **Pre-Execution Summaries**: Show what will happen before making changes
2. **Contextual Tips**: Provide helpful guidance during workflows
3. **Progress Indicators**: Show progress for long-running operations
4. **Confirmation Prompts**: Ask before destructive or irreversible actions

## Command Interface Standards

### Help Output
- Start with a brief, friendly description
- Group related options with clear headers
- Include simple examples for common use cases

### Interactive Prompts
- Present options as vertical lettered lists (A, B, C...)
- Include "Type your own answer" and "Autogenerate" options where appropriate
- Confirm understanding before proceeding

### Error Messages
- State clearly what went wrong
- Explain why (when helpful)
- Provide specific next steps
- Include recovery options when available

## Documentation Structure

Each piece of documentation should:

1. **Start Simple**: Begin with the most common use case
2. **Build Complexity**: Add advanced features progressively
3. **Show Examples**: Real code snippets for every major concept
4. **Summarize**: Recap key points at the end
