
import { getLocale, setLocale, t } from "./i18n.js";

const LARGE_FILE_BYTES = 100 * 1024 * 1024;

const dropZone = document.querySelector("#drop-zone");
const selectButton = document.querySelector("#select-files");
const fileInput = document.querySelector("#file-input");
const clearButton = document.querySelector("#clear-list");
const fileList = document.querySelector("#file-list");
const emptyState = document.querySelector("#empty-state");
const fileSummary = document.querySelector("#file-summary");
const descriptionMeta = document.querySelector('meta[name="description"]');
const languageButtons = document.querySelectorAll("[data-language]");

const records = new Map();
const queue = [];
let nextId = 1;
let activeId = null;
let worker = null;
let workerGeneration = 0;
let dragDepth = 0;

for (const button of languageButtons) {
    button.addEventListener("click", () => setLanguage(button.dataset.language));
}

selectButton.addEventListener("click", (event) => {
    event.stopPropagation();
    fileInput.click();
});

dropZone.addEventListener("click", (event) => {
    if (!event.target.closest("button")) {
        fileInput.click();
    }
});

fileInput.addEventListener("change", () => {
    addFiles(fileInput.files);
    fileInput.value = "";
});

dropZone.addEventListener("dragenter", (event) => {
    event.preventDefault();
    dragDepth += 1;
    dropZone.classList.add("is-dragging");
});

dropZone.addEventListener("dragover", (event) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
});

dropZone.addEventListener("dragleave", (event) => {
    event.preventDefault();
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) {
        dropZone.classList.remove("is-dragging");
    }
});

dropZone.addEventListener("drop", (event) => {
    event.preventDefault();
    dragDepth = 0;
    dropZone.classList.remove("is-dragging");
    addFiles(event.dataTransfer.files);
});

clearButton.addEventListener("click", clearAll);

window.addEventListener("beforeunload", () => {
    revokeAllUrls();
    worker?.terminate();
});

setLanguage("zh");

function setLanguage(language) {
    if (!setLocale(language)) return;

    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    document.title = t("pageTitle");
    descriptionMeta.content = t("metaDescription");

    for (const element of document.querySelectorAll("[data-i18n]")) {
        element.textContent = t(element.dataset.i18n);
    }

    for (const element of document.querySelectorAll("[data-i18n-aria-label]")) {
        element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
    }

    for (const button of languageButtons) {
        button.setAttribute("aria-pressed", String(button.dataset.language === language));
    }

    for (const record of records.values()) renderRecordLanguage(record);
    renderSummary();
}

function addFiles(fileCollection) {
    for (const file of fileCollection) {
        const id = nextId;
        nextId += 1;

        const record = {
            id,
            file,
            originalName: file.name,
            size: file.size,
            state: "waiting",
            error: null,
            result: null,
            objectUrl: null,
            elements: createFileRow(id, file),
        };

        records.set(id, record);
        fileList.append(record.elements.row);
        renderRecordLanguage(record);

        if (!/\.ncm$/i.test(file.name)) {
            failRecord(record, "unsupportedFile");
            continue;
        }

        queue.push(id);
    }

    renderSummary();
    void processNext();
}

function createFileRow(id, file) {
    const row = document.createElement("li");
    row.className = "file-item";
    row.dataset.id = String(id);

    const main = document.createElement("div");
    main.className = "file-main";

    const icon = document.createElement("div");
    icon.className = "file-icon";
    icon.setAttribute("aria-hidden", "true");
    icon.textContent = "NCM";

    const identity = document.createElement("div");

    const name = document.createElement("h3");
    name.className = "file-name";
    name.textContent = file.name;
    name.title = file.name;

    const detail = document.createElement("p");
    detail.className = "file-detail";
    detail.textContent = formatBytes(file.size);

    identity.append(name, detail);

    const status = document.createElement("span");
    status.className = "status";
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    status.textContent = t("statusWaiting");

    main.append(icon, identity, status);
    row.append(main);

    let message = null;
    if (file.size >= LARGE_FILE_BYTES) {
        message = document.createElement("p");
        message.className = "file-message";
        message.textContent = t("largeFile");
        row.append(message);
    }

    const actions = document.createElement("div");
    actions.className = "file-actions";
    actions.hidden = true;
    row.append(actions);

    return { row, detail, status, message, actions, audio: null, download: null };
}

async function processNext() {
    if (activeId !== null || queue.length === 0) {
        return;
    }

    const id = queue.shift();
    const record = records.get(id);
    if (!record) {
        void processNext();
        return;
    }

    activeId = id;
    updateStatus(record, "reading");

    const currentWorker = ensureWorker();
    const generation = workerGeneration;

    try {
        const buffer = await record.file.arrayBuffer();
        record.file = null;

        if (
            activeId !== id ||
            !records.has(id) ||
            generation !== workerGeneration ||
            currentWorker !== worker
        ) {
            return;
        }

        currentWorker.postMessage({ id, buffer }, [buffer]);
    } catch (error) {
        if (records.has(id)) {
            failRecord(record, "readFailed", error instanceof Error ? error.message : "");
            finishActive(id);
        }
    }
}

function ensureWorker() {
    if (worker) {
        return worker;
    }

    workerGeneration += 1;
    const generation = workerGeneration;
    worker = new Worker(new URL("./converter.worker.js", import.meta.url), { type: "module" });

    worker.addEventListener("message", (event) => {
        if (generation !== workerGeneration) {
            return;
        }
        handleWorkerMessage(event.data);
    });

    worker.addEventListener("error", (event) => {
        if (generation !== workerGeneration) {
            return;
        }

        event.preventDefault();
        const affectedIds = [activeId, ...queue];

        queue.length = 0;
        activeId = null;

        for (const id of affectedIds) {
            const record = records.get(id);
            if (record && record.state !== "success" && record.state !== "error") {
                failRecord(record, "workerFailed");
            }
        }

        worker?.terminate();
        worker = null;
        renderSummary();
    });

    return worker;
}

function handleWorkerMessage(message) {
    const record = records.get(message.id);
    if (!record || message.id !== activeId) {
        return;
    }

    if (message.type === "started") {
        updateStatus(record, "working");
        return;
    }

    if (message.type === "error") {
        failRecord(record, "conversionFailed", message.message);
        finishActive(message.id);
        return;
    }

    if (message.type === "success") {
        completeRecord(record, message);
        finishActive(message.id);
    }
}

function completeRecord(record, result) {
    if (record.objectUrl) {
        URL.revokeObjectURL(record.objectUrl);
    }

    const blob = new Blob([result.audioBuffer], { type: result.mimeType });
    record.objectUrl = URL.createObjectURL(blob);
    record.result = result;
    record.error = null;

    const metadata = [result.artist, result.album].filter(Boolean).join(" · ");
    record.elements.detail.textContent = [result.format, formatBytes(blob.size), metadata]
        .filter(Boolean)
        .join(" · ");

    updateStatus(record, "success");

    const audio = document.createElement("audio");
    audio.controls = true;
    audio.preload = "metadata";
    audio.src = record.objectUrl;
    audio.setAttribute("aria-label", t("preview", { name: result.title || record.originalName }));

    const download = document.createElement("a");
    download.className = "download-button";
    download.href = record.objectUrl;
    download.download = buildDownloadName(record.originalName, result);
    download.textContent = t("download", { format: result.format });
    download.setAttribute("aria-label", t("downloadNamed", { name: download.download }));

    record.elements.audio = audio;
    record.elements.download = download;
    record.elements.actions.replaceChildren(audio, download);
    record.elements.actions.hidden = false;
    renderSummary();
}

function failRecord(record, key, details = "") {
    record.error = { key, details: details?.trim() ?? "" };
    updateStatus(record, "error");

    if (!record.elements.message) {
        record.elements.message = document.createElement("p");
        record.elements.row.insertBefore(record.elements.message, record.elements.actions);
    }

    record.elements.message.className = "file-message is-error";
    renderRecordLanguage(record);
    renderSummary();
}

function finishActive(id) {
    if (activeId === id) {
        activeId = null;
    }
    renderSummary();
    void processNext();
}

function updateStatus(record, state) {
    record.state = state;
    record.elements.status.className = "status";

    if (state === "reading" || state === "working") {
        record.elements.status.classList.add("is-working");
    } else if (state === "success") {
        record.elements.status.classList.add("is-success");
    } else if (state === "error") {
        record.elements.status.classList.add("is-error");
    }

    renderRecordLanguage(record);
    renderSummary();
}

function renderRecordLanguage(record) {
    const statusKeys = {
        waiting: "statusWaiting",
        reading: "statusReading",
        working: "statusWorking",
        success: "statusSuccess",
        error: "statusError",
    };
    record.elements.status.textContent = t(statusKeys[record.state]);

    if (record.elements.message) {
        if (record.error) {
            const fallback = record.error.key === "conversionFailed" ? t("damagedFile") : "";
            const details = record.error.details || fallback;
            const separator = getLocale() === "zh" ? "：" : ": ";
            record.elements.message.textContent = details
                ? t(record.error.key) + separator + details
                : t(record.error.key);
        } else {
            record.elements.message.textContent = t("largeFile");
        }
    }

    if (record.result && record.elements.audio && record.elements.download) {
        const displayName = record.result.title || record.originalName;
        record.elements.audio.setAttribute("aria-label", t("preview", { name: displayName }));
        record.elements.download.textContent = t("download", { format: record.result.format });
        record.elements.download.setAttribute(
            "aria-label",
            t("downloadNamed", { name: record.elements.download.download }),
        );
    }
}

function renderSummary() {
    const values = [...records.values()];
    emptyState.hidden = values.length > 0;
    clearButton.disabled = values.length === 0;

    if (values.length === 0) {
        fileSummary.textContent = t("emptySummary");
        return;
    }

    const success = values.filter((record) => record.state === "success").length;
    const failed = values.filter((record) => record.state === "error").length;
    const inProgress = values.filter(
        (record) => record.state === "reading" || record.state === "working",
    ).length;
    const waiting = values.length - success - failed - inProgress;

    const total =
        getLocale() === "zh"
            ? "共 " + values.length + " 个"
            : values.length + " " + (values.length === 1 ? "file" : "files") + " total";
    const parts = [total];
    if (waiting) parts.push(t("summaryWaiting", { count: waiting }));
    if (inProgress) parts.push(t("summaryWorking", { count: inProgress }));
    if (success) parts.push(t("summarySuccess", { count: success }));
    if (failed) parts.push(t("summaryFailed", { count: failed }));
    fileSummary.textContent = parts.join(" · ");
}

function clearAll() {
    workerGeneration += 1;
    worker?.terminate();
    worker = null;
    queue.length = 0;
    activeId = null;

    revokeAllUrls();
    records.clear();
    fileList.replaceChildren();
    renderSummary();
    selectButton.focus();
}

function revokeAllUrls() {
    for (const record of records.values()) {
        if (record.objectUrl) {
            URL.revokeObjectURL(record.objectUrl);
            record.objectUrl = null;
        }
    }
}

function buildDownloadName(originalName, result) {
    const originalStem = originalName.replace(/\.ncm$/i, "");
    const metadataStem = result.title
        ? [result.artist, result.title].filter(Boolean).join(" - ")
        : originalStem;
    const safeStem =
        metadataStem
            .normalize("NFC")
            .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_")
            .replace(/[. ]+$/g, "")
            .trim()
            .slice(0, 180) || "converted";

    return `${safeStem}.${result.extension}`;
}

function formatBytes(bytes) {
    if (bytes === 0) return "0 B";

    const units = ["B", "KB", "MB", "GB"];
    const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / 1024 ** unit;
    return `${value.toFixed(unit === 0 || value >= 10 ? 0 : 1)} ${units[unit]}`;
}

function readableError(error, fallback) {
    if (error instanceof Error && error.message) {
        return `${fallback} ${error.message}`;
    }
    return fallback;
}
