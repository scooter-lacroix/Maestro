/**
 * TrackLens UI - Completion Overlay Component
 *
 * Shows approval/denial/feedback completion screen with auto-close support.
 *
 * REBRANDED: Plannotator → TrackLens
 *
 * @packageDocumentation
 */

import { useAutoClose } from '../hooks/useAutoClose';

const CheckIcon = () => (
  <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
  </svg>
);

const ChatBubbleIcon = () => (
  <svg className="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
    />
  </svg>
);

interface CompletionOverlayProps {
  submitted: 'approved' | 'denied' | 'feedback' | null | false;
  title: string;
  subtitle: string;
  agentLabel: string;
  onDismiss?: () => void;
}

export function CompletionOverlay({ submitted, title, subtitle, agentLabel, onDismiss }: CompletionOverlayProps) {
  const { state, enableAndStart } = useAutoClose(!!submitted);

  if (!submitted) return null;

  const isApproved = submitted === 'approved';

  return (
    <div className="fixed inset-0 z-[100] bg-background flex items-center justify-center">
      <div className="text-center space-y-6 max-w-md px-8">
        <div
          className={`mx-auto w-16 h-16 rounded-full flex items-center justify-center ${
            isApproved ? 'bg-green-500/20 text-green-500' : 'bg-blue-500/20 text-blue-500'
          }`}
        >
          {isApproved ? <CheckIcon /> : <ChatBubbleIcon />}
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold text-foreground">{title}</h2>
            {onDismiss && (
              <button
                onClick={onDismiss}
                className="p-2 rounded-lg hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
                title="Close"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            )}
          </div>
          <p className="text-muted-foreground">{subtitle}</p>
        </div>

        <div className="pt-4 border-t border-border space-y-2">
          {state.phase === 'counting' ? (
            <>
              <p className="text-sm text-muted-foreground">
                This tab will close in <span className="text-foreground font-medium">{state.remaining}</span> second
                {state.remaining !== 1 ? 's' : ''}...
              </p>
              <p className="text-xs text-muted-foreground/60">You can change this in Settings.</p>
            </>
          ) : state.phase === 'closeFailed' ? (
            <>
              <p className="text-sm text-muted-foreground">
                Could not close this tab automatically. Please close it manually.
              </p>
              <p className="text-xs text-muted-foreground/60">
                Auto-close works when the tab is opened by {agentLabel}.
              </p>
            </>
          ) : (
            <>
              <p className="text-sm text-muted-foreground">
                You can close this tab and return to {agentLabel}.
              </p>
              {state.phase === 'prompt' ? (
                <>
                  <label className="flex items-center justify-center gap-2 cursor-pointer group">
                    <input type="checkbox" checked={false} onChange={enableAndStart} className="accent-primary" />
                    <span className="text-xs text-muted-foreground group-hover:text-foreground transition-colors">
                      Auto-close this tab after 3 seconds
                    </span>
                  </label>
                  <p className="text-xs text-muted-foreground/60">You can change the delay in Settings.</p>
                </>
              ) : (
                <p className="text-xs text-muted-foreground/60">Your response has been sent.</p>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
