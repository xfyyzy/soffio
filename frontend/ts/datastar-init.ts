import { apply, load, ActionPlugin } from './datastar.js';

type UploadStatus = 'pending' | 'uploading' | 'success' | 'error';

interface UploadQueueEntry {
  id: string | null;
  name: string;
  file: File | null;
  sizeBytes: number;
  status: UploadStatus;
  message: string | null;
}

interface UploadQueueState {
  entries: UploadQueueEntry[];
  processing: boolean;
}

interface QueueDom {
  container: HTMLElement;
  body: HTMLElement | null;
  manifest: HTMLInputElement | null;
  fileInput: HTMLInputElement | null;
}

interface UploadEntryEventDetail {
  id: string;
  status?: UploadStatus;
  message?: string | null;
  sizeBytes?: number;
  suppressPanel?: boolean;
}

interface WorkerForms {
  form: HTMLFormElement;
  idField: HTMLInputElement;
  fileField: HTMLInputElement;
}

interface ActionContext {
  el?: Element | null;
}

type ActionParams = Record<string, unknown>;
type ActionHandler = (...args: unknown[]) => void | Promise<void>;

type ActionHandlerMap = Record<string, ActionHandler>;

const uploadQueues = new WeakMap<HTMLFormElement, UploadQueueState>();
let activeQueueForm: HTMLFormElement | null = null;

const getFormElements = (form: HTMLFormElement | null): HTMLFormControlsCollection | null => {
  if (!form || typeof form.elements === 'undefined') {
    return null;
  }
  return form.elements;
};

const setFormFieldValue = (form: HTMLFormElement | null, name: string, value: string): void => {
  const elements = getFormElements(form);
  if (!elements || typeof elements.namedItem !== 'function') {
    return;
  }
  const field = elements.namedItem(name) as HTMLInputElement | null;
  if (field) {
    field.value = value;
  }
};

const submitForm = (form: HTMLFormElement | null): void => {
  if (form && typeof form.requestSubmit === 'function') {
    form.requestSubmit();
  }
};

const resolveClosestForm = (el: Element | EventTarget | null): HTMLFormElement | null => {
  if (!el) {
    return null;
  }
  if (el instanceof HTMLFormElement) {
    return el;
  }
  if (el instanceof Element && typeof el.closest === 'function') {
    const form = el.closest('form');
    if (form instanceof HTMLFormElement) {
      return form;
    }
  }
  return null;
};

const generateQueueId = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  const seed = Math.random().toString(36).slice(2, 10);
  return `upload-${Date.now().toString(36)}-${seed}`;
};

const ensureUploadQueue = (form: HTMLFormElement): UploadQueueState => {
  if (!uploadQueues.has(form)) {
    uploadQueues.set(form, { entries: [], processing: false });
  }
  activeQueueForm = form;
  return uploadQueues.get(form)!;
};

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }
  const thresholds = [
    { limit: 1024 ** 3, suffix: 'GiB' },
    { limit: 1024 ** 2, suffix: 'MiB' },
    { limit: 1024, suffix: 'KiB' },
  ];
  for (const { limit, suffix } of thresholds) {
    if (bytes >= limit) {
      const value = bytes / limit;
      return `${value.toFixed(bytes >= 10 * limit ? 0 : 1)} ${suffix}`;
    }
  }
  return `${bytes} B`;
};

const getQueueDom = (form: HTMLFormElement): QueueDom | null => {
  const container = form.querySelector<HTMLElement>('[data-upload-queue]');
  if (!container) {
    return null;
  }
  return {
    container,
    body: container.querySelector<HTMLElement>('[data-role="upload-queue-body"]'),
    manifest: form.querySelector<HTMLInputElement>('input[name="queue_manifest"]'),
    fileInput: form.querySelector<HTMLInputElement>('input[type="file"][name="file"]'),
  };
};

const updateManifestInput = (dom: QueueDom, queue: UploadQueueState): string => {
  const manifestField = dom.manifest;
  if (!manifestField) {
    return '[]';
  }
  try {
    const manifest = queue.entries.map((entry) => ({
      id: entry.id,
      filename: entry.name,
      size_bytes: entry.sizeBytes,
      status: entry.status,
      message: entry.message ?? null,
    }));
    const value = JSON.stringify(manifest);
    manifestField.value = value;
    return value;
  } catch (error) {
    void error;
    return '[]';
  }
};

const updateInputFiles = (dom: QueueDom, queue: UploadQueueState): void => {
  const fileInput = dom.fileInput;
  if (!fileInput) {
    return;
  }
  if (typeof DataTransfer === 'undefined') {
    if (queue.entries.every((entry) => !(entry.file instanceof File))) {
      try {
        fileInput.value = '';
      } catch (error) {
        void error;
      }
    }
    return;
  }
  const transfer = new DataTransfer();
  for (const entry of queue.entries) {
    if (entry.file instanceof File) {
      transfer.items.add(entry.file);
    }
  }
  fileInput.files = transfer.files;
  if (transfer.files.length === 0) {
    try {
      fileInput.value = '';
    } catch (error) {
      void error;
    }
  }
};

const UPLOAD_QUEUE_EVENT = 'admin:upload-entry';

const findAuxForm = (form: HTMLFormElement, role: string): HTMLFormElement | null => {
  const body = form.parentElement ?? form.closest<HTMLElement>('[data-role="panel-body"]');
  if (!body) {
    return null;
  }
  const node = body.querySelector(`[data-role="${role}"]`);
  return node instanceof HTMLFormElement ? node : null;
};

const refreshQueueBindings = (form: HTMLFormElement, state: UploadQueueState): HTMLFormElement | null => {
  const dom = getQueueDom(form);
  if (!dom) {
    return null;
  }
  const manifestValue = updateManifestInput(dom, state);
  updateInputFiles(dom, state);
  const syncForm = findAuxForm(form, 'upload-queue-sync');
  if (syncForm) {
    const manifestField = syncForm.querySelector<HTMLInputElement>('input[name="queue_manifest"]');
    if (manifestField) {
      manifestField.value = manifestValue;
    }
  }
  if (dom.body) {
    apply(dom.body);
  }
  return syncForm;
};

const postQueueSnapshot = (form: HTMLFormElement, state: UploadQueueState): void => {
  const syncForm = refreshQueueBindings(form, state);
  if (syncForm) {
    syncForm.requestSubmit();
  }
};

const waitForEntryResult = (entryId: string, suppressPanel: boolean): Promise<UploadEntryEventDetail> =>
  new Promise((resolve) => {
    const handler: EventListener = (event) => {
      const customEvent = event as CustomEvent<UploadEntryEventDetail>;
      const detail = customEvent.detail;
      if (!detail || detail.id !== entryId) {
        return;
      }
      if (typeof detail.suppressPanel === 'boolean' && detail.suppressPanel !== suppressPanel) {
        return;
      }
      window.removeEventListener(UPLOAD_QUEUE_EVENT, handler);
      resolve(detail);
    };
    window.addEventListener(UPLOAD_QUEUE_EVENT, handler);
  });

const configureUploadWorker = (form: HTMLFormElement, entry: UploadQueueEntry): WorkerForms | null => {
  const workerForm = findAuxForm(form, 'upload-queue-upload');
  if (!workerForm) {
    return null;
  }

  const idField = workerForm.querySelector<HTMLInputElement>('input[name="queue_entry_id"]');
  const fileField = workerForm.querySelector<HTMLInputElement>('input[type="file"][name="file"]');
  if (!idField || !fileField) {
    return null;
  }

  idField.value = entry.id ?? '';

  if (entry.file instanceof File && typeof DataTransfer !== 'undefined') {
    const transfer = new DataTransfer();
    transfer.items.add(entry.file);
    fileField.files = transfer.files;
  } else {
    fileField.value = '';
  }

  return { form: workerForm, idField, fileField };
};

window.addEventListener(UPLOAD_QUEUE_EVENT, (event) => {
  const detail = (event as CustomEvent<UploadEntryEventDetail>).detail;
  if (!detail || !detail.id) {
    return;
  }
  const form = activeQueueForm;
  if (!form) {
    return;
  }
  const state = uploadQueues.get(form);
  if (!state) {
    return;
  }
  const entry = state.entries.find((item) => item.id === detail.id);
  if (!entry) {
    return;
  }
  if (typeof detail.status === 'string') {
    entry.status = detail.status;
  }
  entry.message = typeof detail.message === 'string' && detail.message.trim().length > 0 ? detail.message : null;
  if (typeof detail.sizeBytes === 'number' && Number.isFinite(detail.sizeBytes)) {
    entry.sizeBytes = detail.sizeBytes;
  }
  entry.file = null;
  postQueueSnapshot(form, state);
});

const actionHandlers: ActionHandlerMap = {
  async copyToClipboard(ctxArg, paramsArg = {}) {
    const { el } = (ctxArg ?? {}) as ActionContext;
    const {
      href,
      successMessage = 'Link copied to clipboard',
      errorMessage = 'Copy failed',
      kindField = 'kind',
      messageField = 'message',
    } = (paramsArg ?? {}) as {
      href?: string;
      successMessage?: string;
      errorMessage?: string;
      kindField?: string;
      messageField?: string;
    };

    if (!href) {
      return;
    }

    const form = resolveClosestForm(el ?? null);
    if (!form) {
      return;
    }

    const setToast = (kind: string, message: string): void => {
      setFormFieldValue(form, kindField, kind);
      if (typeof message === 'string') {
        setFormFieldValue(form, messageField, message);
      }
    };

    try {
      if (!navigator.clipboard || typeof navigator.clipboard.writeText !== 'function') {
        throw new Error('Clipboard API unavailable');
      }
      await navigator.clipboard.writeText(href);
      setToast('success', successMessage ?? 'Link copied to clipboard');
    } catch (error) {
      void error;
      setToast('error', errorMessage ?? 'Copy failed');
    }

    submitForm(form);
  },

  queueFiles(ctxArg) {
    const { el } = (ctxArg ?? {}) as ActionContext;
    if (!(el instanceof HTMLInputElement)) {
      return;
    }
    const form = resolveClosestForm(el);
    if (!form) {
      return;
    }
    const state = ensureUploadQueue(form);
    if (state.processing) {
      return;
    }
    const files = el.files ? Array.from(el.files) : [];
    state.entries = files.map<UploadQueueEntry>((file) => ({
      id: generateQueueId(),
      name: file.name,
      file,
      sizeBytes: file.size,
      status: 'pending',
      message: null,
    }));
    postQueueSnapshot(form, state);
  },

  removeQueuedFile(ctxArg, paramsArg = {}) {
    const { el } = (ctxArg ?? {}) as ActionContext;
    const form = resolveClosestForm(el ?? null);
    if (!form) {
      return;
    }
    const state = ensureUploadQueue(form);
    if (state.processing) {
      return;
    }
    const rawParams = (paramsArg ?? {}) as ActionParams;
    const id = typeof rawParams?.id === 'string' ? (rawParams.id as string) : undefined;
    if (!id) {
      return;
    }
    const index = state.entries.findIndex((entry) => entry.id === id);
    if (index === -1 || state.entries[index]?.status === 'uploading') {
      return;
    }
    state.entries.splice(index, 1);
    postQueueSnapshot(form, state);
  },

  async processUploadQueue(ctxArg) {
    const { el } = (ctxArg ?? {}) as ActionContext;
    const form = resolveClosestForm(el ?? null);
    if (!form) {
      return;
    }
    const state = ensureUploadQueue(form);
    if (state.processing) {
      return;
    }

    state.processing = true;
    try {
      const limitAttr = form.dataset?.uploadLimit;
      const limit = Number(limitAttr);
      const limitBytes = Number.isFinite(limit) ? limit : null;
      let mutated = false;

      if (limitBytes !== null) {
        for (const entry of state.entries) {
          if (entry.file instanceof File && entry.file.size > limitBytes) {
            if (entry.status !== 'error' || entry.message === null) {
              entry.status = 'error';
              entry.message = `File exceeds limit (${formatBytes(limitBytes)})`;
              mutated = true;
            }
          } else if (entry.status === 'error' && entry.file instanceof File) {
            entry.status = 'pending';
            entry.message = null;
            mutated = true;
          }
        }
        if (mutated) {
          postQueueSnapshot(form, state);
        }
      }

      for (const entry of state.entries) {
        if (!(entry.file instanceof File)) {
          continue;
        }
        if (entry.status === 'error' || entry.status === 'success') {
          continue;
        }
        if (limitBytes !== null && entry.file.size > limitBytes) {
          continue;
        }

        if (!entry.id) {
          entry.id = generateQueueId();
        }

        entry.status = 'uploading';
        entry.message = null;
        postQueueSnapshot(form, state);

        const worker = configureUploadWorker(form, entry);
        if (!worker) {
          entry.status = 'error';
          entry.message = 'Upload form unavailable';
          postQueueSnapshot(form, state);
          continue;
        }

        const suppressPanel = Boolean(worker.form.querySelector("input[name=\"suppress_panel_patch\"]"));
        const resultPromise = waitForEntryResult(entry.id, suppressPanel);
        worker.form.requestSubmit();
        await resultPromise;
        worker.idField.value = '';
        worker.fileField.value = '';
      }
    } finally {
      state.processing = false;
      postQueueSnapshot(form, state);
    }
  },
};

const actionPlugins: ActionPlugin[] = Object.entries(actionHandlers).map(([name, handler]) => ({
  type: 'action',
  name,
  fn: handler,
}));

if (actionPlugins.length > 0) {
  load(...actionPlugins);
}

load();
