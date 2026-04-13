/**
 * MarkdownEditor - CodeMirror-based markdown editor
 *
 * Simple wrapper around CodeMirror for inline editing of markdown content.
 */

import { useEffect, useRef } from 'react';
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { basicSetup } from 'codemirror';
import type { ViewUpdate } from '@codemirror/view';

interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
}

export function MarkdownEditor({ value, onChange }: MarkdownEditorProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);

  useEffect(() => {
    if (!editorRef.current) return;

    // Create editor state
    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        markdown(),
        EditorView.theme({
          '&': {
            backgroundColor: 'transparent',
            color: 'inherit',
          },
          '.cm-content': {
            padding: '0',
            fontFamily: 'inherit',
            fontSize: 'inherit',
            lineHeight: 'inherit',
          },
          '.cm-line': {
            padding: '0',
          },
          '.cm-editor': {
            height: '100%',
          },
          '.cm-scroller': {
            fontFamily: 'inherit',
            fontSize: '14px',
            lineHeight: '1.7',
          },
        }),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update: ViewUpdate) => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
      ],
    });

    // Create editor view
    const view = new EditorView({
      state,
      parent: editorRef.current,
    });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []);

  // Update document when value changes externally
  useEffect(() => {
    const view = viewRef.current;
    if (view && view.state.doc.toString() !== value) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: value,
        },
      });
    }
  }, [value]);

  return (
    <div
      ref={editorRef}
      className="h-full w-full focus:outline-none"
      style={{ minHeight: '400px' }}
    />
  );
}
