import "@deepseek-ai/dsh-launch-environment";
import "@deepseek-ai/dsh-credentials";
import z from "@deepseek-ai/schemastery";
import "@deepseek-ai/dsh-llm";
import { Context, Logger } from "@deepseek-ai/cordis";
import "@deepseek-ai/dsh-agent";
//#region ../core/dist/types.d.ts
type JsonPrimitive = string | number | boolean | null;
type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
interface JsonObject {
  [key: string]: JsonValue;
}
type MemosInfo = Record<string, string>;
interface MemosToolCall {
  id: string;
  type: 'function';
  function: {
    name: string;
    arguments: string;
  };
}
interface MemosTextContentBlock {
  type: 'text';
  text: string;
}
interface MemosMessageMetadata {
  id?: string;
  chat_time?: string;
}
interface MemosUserMessage extends MemosMessageMetadata {
  role: 'user';
  content: string;
}
interface MemosAssistantMessage extends MemosMessageMetadata {
  role: 'assistant';
  content?: string;
  tool_calls?: MemosToolCall[];
}
interface MemosToolMessage extends MemosMessageMetadata {
  role: 'tool';
  tool_call_id: string;
  content: MemosTextContentBlock[];
}
type MemosMessage = MemosUserMessage | MemosAssistantMessage | MemosToolMessage;
interface MemosSearchRequest {
  user_id: string;
  query: string;
  conversation_id?: string;
  source: string;
  memory_limit_number: number;
  include_preference: boolean;
  preference_limit_number: number;
  include_tool_memory: boolean;
  tool_memory_limit_number: number;
  relativity: number;
  filter?: JsonObject;
  knowledgebase_ids?: string[];
}
interface MemosAddRequest {
  user_id: string;
  conversation_id: string;
  messages: MemosMessage[];
  source: string;
  async_mode: boolean;
  allow_public: boolean;
  tags?: string[];
  info?: MemosInfo;
  agent_id?: string;
  app_id?: string;
  allow_knowledgebase_ids?: string[];
}
interface MemosMemoryDetail {
  id?: string;
  memory_key?: string;
  memory_value?: string;
  memory_type?: string;
  create_time?: number | string;
  update_time?: number | string;
  relativity?: number;
  tags?: string[];
  [key: string]: JsonValue | undefined;
}
interface MemosPreferenceDetail {
  id?: string;
  memory_value?: string;
  preference?: string;
  preference_type?: string;
  reasoning?: string;
  create_time?: number | string;
  update_time?: number | string;
  relativity?: number;
  [key: string]: JsonValue | undefined;
}
interface MemosToolMemoryDetail {
  id?: string;
  tool_type?: string;
  tool_value?: string;
  experience?: string;
  create_time?: number | string;
  update_time?: number | string;
  relativity?: number;
  [key: string]: JsonValue | undefined;
}
interface MemosSearchData {
  memory_detail_list?: MemosMemoryDetail[];
  preference_detail_list?: MemosPreferenceDetail[];
  tool_memory_detail_list?: MemosToolMemoryDetail[];
  preference_note?: string;
}
interface MemosAddData {
  success?: boolean;
  task_id?: string;
  status?: string;
}
interface BaseResponse<T> {
  code: number | string;
  data?: T;
  message?: string;
}
//#endregion
//#region ../core/dist/filter.d.ts
declare const isPerSourceFilter: (value: JsonObject) => boolean;
declare function validateFilter(value: unknown, knowledgebaseIds?: readonly string[]): asserts value is JsonObject | undefined;
declare const buildEffectiveFilter: (filter: JsonObject | undefined, agentId: string | undefined) => JsonObject | undefined;
//#endregion
//#region ../core/dist/memos-errors.d.ts
type MemosClientErrorKind = 'credential' | 'http' | 'business' | 'response' | 'network' | 'timeout' | 'aborted';
interface MemosClientErrorOptions {
  kind: MemosClientErrorKind;
  retryable?: boolean;
  status?: number;
  cause?: unknown;
}
declare class MemosClientError extends Error {
  readonly kind: MemosClientErrorKind;
  readonly retryable: boolean;
  readonly status?: number;
  constructor(message: string, options: MemosClientErrorOptions);
}
//#endregion
//#region ../core/dist/memos-client.d.ts
interface MemosClientOptions {
  baseURL: string;
  timeoutMs: number;
  searchRetries: number;
  addRetries?: number;
  retryDelayMs?: number;
  resolveApiKey: () => Promise<string | undefined>;
  fetch?: typeof globalThis.fetch;
  lifecycleSignal?: AbortSignal;
  redirect?: RequestRedirect;
  retryAllFailures?: boolean;
}
declare class MemosClient {
  #private;
  constructor(options: MemosClientOptions);
  search(request: MemosSearchRequest, signal?: AbortSignal): Promise<MemosSearchData>;
  add(request: MemosAddRequest, signal?: AbortSignal): Promise<MemosAddData>;
  requestRaw<T = unknown>(path: string, body: unknown, retries: number, signal?: AbortSignal): Promise<T>;
}
//#endregion
//#region ../core/dist/memos-response.d.ts
declare const parseSearchResponse: (response: Response, apiKey: string) => Promise<MemosSearchData>;
declare const parseAddResponse: (response: Response, apiKey: string) => Promise<MemosAddData>;
//#endregion
//#region ../core/dist/payloads.d.ts
interface BuildMemosSearchRequestInput {
  userId: string;
  query: string;
  source: string;
  conversationId?: string;
  memoryLimitNumber: number;
  includePreference: boolean;
  preferenceLimitNumber: number;
  includeToolMemory: boolean;
  toolMemoryLimitNumber: number;
  relativity: number;
  filter?: JsonObject;
  knowledgebaseIds?: readonly string[];
}
declare const buildMemosSearchRequest: (input: BuildMemosSearchRequestInput) => MemosSearchRequest;
interface BuildMemosAddRequestInput {
  userId: string;
  conversationId: string;
  messages: readonly MemosMessage[];
  source: string;
  asyncMode: boolean;
  allowPublic: boolean;
  tags?: readonly string[];
  info?: MemosInfo;
  agentId?: string;
  appId?: string;
  allowKnowledgebaseIds?: readonly string[];
}
declare const buildMemosAddRequest: (input: BuildMemosAddRequestInput) => MemosAddRequest;
//#endregion
//#region ../core/dist/recall-projection.d.ts
interface RecallLimits {
  maxItemChars: number;
  maxTotalChars: number;
}
interface RecallFact {
  text: string;
  id?: string;
  createdAt?: number | string;
  updatedAt?: number | string;
  relativity?: number;
}
interface RecallPreference {
  text: string;
  id?: string;
  type?: string;
  createdAt?: number | string;
  updatedAt?: number | string;
  relativity?: number;
}
interface RecallSupplement {
  text: string;
  id?: string;
  type?: string;
  createdAt?: number | string;
  updatedAt?: number | string;
  relativity?: number;
}
interface RecallProjection {
  facts: RecallFact[];
  preferences: RecallPreference[];
  tools?: RecallSupplement[];
}
declare const projectRecall: (data: MemosSearchData, limits: RecallLimits) => RecallProjection | undefined;
//#endregion
//#region ../core/dist/serialization.d.ts
declare const stringifyTagSafeJson: (value: JsonValue) => string;
declare const unicodeLength: (value: string) => number;
declare const truncateUnicode: (value: string, maxChars: number) => string;
//#endregion
//#region src/config.d.ts
interface Config {
  apiKey?: string;
  apiKeyEnv?: string;
  baseURL?: string;
  userId?: string;
  recallEnabled?: boolean;
  addEnabled?: boolean;
  includeAssistant?: boolean;
  includeSubagents?: boolean;
  multiAgentMode?: boolean;
  queryPrefix?: string;
  recallGlobal?: boolean;
  memoryLimitNumber?: number;
  preferenceLimitNumber?: number;
  includePreference?: boolean;
  includeToolMemory?: boolean;
  toolMemoryLimitNumber?: number;
  relativity?: number;
  filter?: JsonObject;
  knowledgebaseIds?: string[];
  tags?: string[];
  info?: MemosInfo;
  agentId?: string;
  appId?: string;
  allowKnowledgebaseIds?: string[];
  maxQueryChars?: number;
  maxRecallChars?: number;
  maxItemChars?: number;
  maxMessageChars?: number;
  timeoutMs?: number;
  searchRetries?: number;
  addRetries?: number;
  allowPublic?: boolean;
  asyncMode?: boolean;
}
interface ResolvedConfig {
  apiKey?: string;
  apiKeyEnv: string;
  baseURL: string;
  userId: string;
  recallEnabled: boolean;
  addEnabled: boolean;
  includeAssistant: boolean;
  includeSubagents: boolean;
  multiAgentMode: boolean;
  queryPrefix: string;
  recallGlobal: boolean;
  memoryLimitNumber: number;
  preferenceLimitNumber: number;
  includePreference: boolean;
  includeToolMemory: boolean;
  toolMemoryLimitNumber: number;
  relativity: number;
  filter?: JsonObject;
  knowledgebaseIds: string[];
  tags: string[];
  info: MemosInfo;
  agentId?: string;
  appId?: string;
  allowKnowledgebaseIds: string[];
  maxQueryChars: number;
  maxRecallChars: number;
  maxItemChars: number;
  maxMessageChars: number;
  timeoutMs: number;
  searchRetries: number;
  addRetries: number;
  allowPublic: boolean;
  asyncMode: boolean;
}
declare const Config: z<Config>;
//#endregion
//#region src/lifecycle-types.d.ts
interface MemosClientLike {
  search(request: MemosSearchRequest, signal?: AbortSignal): Promise<MemosSearchData>;
  add(request: MemosAddRequest, signal?: AbortSignal): Promise<unknown>;
}
interface MemosLifecycleDependencies {
  clientFactory?: (config: ResolvedConfig, lifecycleSignal: AbortSignal) => MemosClientLike;
  logger?: Pick<Logger, 'warn'>;
  platform?: NodeJS.Platform;
  maxRecallItemChars?: number;
}
interface MemosLifecycleController {
  drain(): Promise<void>;
  dispose(): Promise<void>;
}
//#endregion
//#region src/lifecycle.d.ts
declare const MEMOS_SETTINGS_NAMESPACE: string;
declare const installMemosLifecycle: (ctx: Context, entry?: Config, dependencies?: MemosLifecycleDependencies) => MemosLifecycleController;
//#endregion
//#region src/index.d.ts
declare const name = "memos-cloud";
declare const inject: string[];
declare const apply: (ctx: Context, config: Config) => void;
//#endregion
export { type BaseResponse, type BuildMemosAddRequestInput, type BuildMemosSearchRequestInput, Config, type Config as MemosCloudConfig, type JsonObject, type JsonPrimitive, type JsonValue, MEMOS_SETTINGS_NAMESPACE, type MemosAddData, type MemosAddRequest, type MemosAssistantMessage, type MemosClient, type MemosClientError, type MemosClientErrorKind, type MemosClientErrorOptions, type MemosClientLike, type MemosClientOptions, type MemosInfo, type MemosLifecycleController, type MemosLifecycleDependencies, type MemosMemoryDetail, type MemosMessage, type MemosPreferenceDetail, type MemosSearchData, type MemosSearchRequest, type MemosTextContentBlock, type MemosToolCall, type MemosToolMemoryDetail, type MemosToolMessage, type MemosUserMessage, type RecallFact, type RecallLimits, type RecallPreference, type RecallProjection, type RecallSupplement, type ResolvedConfig, apply, type buildEffectiveFilter, type buildMemosAddRequest, type buildMemosSearchRequest, inject, installMemosLifecycle, type isPerSourceFilter, name, type parseAddResponse, type parseSearchResponse, type projectRecall, type stringifyTagSafeJson, type truncateUnicode, type unicodeLength, type validateFilter };
//# sourceMappingURL=index.d.ts.map