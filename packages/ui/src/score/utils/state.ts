import type { AgentStatus, DocumentContext, Version } from '../../generated/protocol.ts';

export interface AppState {
  documentId: string | null;
  connection: 'connecting' | 'connected' | 'reconnecting';
  context: DocumentContext | null;
  versions: Version[];
  versionsReady: boolean;
  versionsLoading: boolean;
  status: AgentStatus | null;
  statusLoading: boolean;
}
export const initialState: AppState = {
  documentId: null,
  connection: 'connecting',
  context: null,
  versions: [],
  versionsReady: false,
  versionsLoading: false,
  status: null,
  statusLoading: false,
};
export type Action =
  | { type: 'start'; documentId: string | null }
  | { type: 'status'; status: AgentStatus }
  | { type: 'status-error' }
  | { type: 'versions'; versions: Version[] }
  | { type: 'versions-error' }
  | { type: 'missing' }
  | { type: 'context'; context: DocumentContext }
  | { type: 'connection'; connection: AppState['connection'] };

/** Keep independent loading results without replacing an already displayed score. */
export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'start':
      return {
        ...state,
        ...(state.documentId === action.documentId
          ? {}
          : {
              documentId: action.documentId,
              context: null,
              versions: [],
              versionsReady: false,
            }),
        versionsLoading: !!action.documentId,
        statusLoading: true,
      };
    case 'status':
      return { ...state, status: action.status, statusLoading: false };
    case 'status-error':
      return { ...state, statusLoading: false };
    case 'versions':
      return { ...state, versions: action.versions, versionsReady: true, versionsLoading: false };
    case 'versions-error':
      return { ...state, versionsLoading: false };
    case 'missing':
      return { ...state, context: null, versions: [], versionsReady: false };
    case 'context':
      return { ...state, context: action.context };
    case 'connection':
      return { ...state, connection: action.connection };
  }
}
