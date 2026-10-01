const COPY = {
    zh: {
        pageTitle: "NCM 浏览器转换器",
        metaDescription: "在浏览器本地将网易云音乐 NCM 文件转换为原始音频，不上传文件。",
        skipLink: "跳到转换区域",
        languageLabel: "Language",
        heading: "NCM 浏览器转换器",
        heroCopy: "将 .ncm 文件还原为 MP3、FLAC、M4A 或 OGG，转换完成后即可试听和下载。",
        privacyBadge: "本地处理 · 不会上传",
        converterTitle: "转换 NCM 文件",
        dropTitle: "拖放 .ncm 文件到这里",
        dropCopy: "支持一次选择多个文件，也可以点击下方按钮",
        selectFiles: "选择文件",
        fileInputLabel: "选择 NCM 文件",
        memoryGuidance:
            "大文件会同时占用浏览器内存进行解密；设备内存不足时请减少同时选择的文件数量。",
        filesHeading: "转换文件",
        clearList: "清空列表",
        emptyState: "选择文件后，转换状态和下载操作会显示在这里。",
        fileListLabel: "转换文件列表",
        disclaimerTitle: "免责声明",
        disclaimerText:
            "请仅将本工具用于您有权解密、转换或以其他方式处理的音频文件。因使用本工具而产生的一切法律责任及其他后果均由使用者自行承担，与工具开发者及贡献者无关。",
        footerPrivacy: "所有文件只在当前浏览器标签页内处理，不会发送到任何服务器。",
        builtFrom: "基于 MIT 许可的",
        builtSuffix: "构建",
        viewSource: "查看源码",
        emptySummary: "尚未添加文件",
        statusWaiting: "等待中",
        statusReading: "正在读取",
        statusWorking: "转换中",
        statusSuccess: "已完成",
        statusError: "失败",
        largeFile:
            "这是一个较大的文件，转换时可能需要约为文件数倍的可用内存，请保持此页面处于打开状态。",
        unsupportedFile: "仅支持扩展名为 .ncm 的文件。",
        readFailed: "无法读取这个文件",
        workerFailed: "转换模块加载失败，请刷新页面后重试。",
        conversionFailed: "转换失败",
        damagedFile: "文件可能已损坏，或不是受支持的 NCM 文件。",
        preview: "试听 {name}",
        download: "下载 {format}",
        downloadNamed: "下载 {name}",
        summaryWaiting: "等待 {count}",
        summaryWorking: "处理中 {count}",
        summarySuccess: "成功 {count}",
        summaryFailed: "失败 {count}",
    },
    en: {
        pageTitle: "NCM Browser Converter",
        metaDescription:
            "Convert NetEase Cloud Music NCM files to their original audio format locally in your browser. Files are never uploaded.",
        skipLink: "Skip to converter",
        languageLabel: "语言",
        heading: "NCM Browser Converter",
        heroCopy:
            "Restore .ncm files to MP3, FLAC, M4A, or OGG, then preview and download the converted audio.",
        privacyBadge: "Local only · No uploads",
        converterTitle: "Convert NCM files",
        dropTitle: "Drop .ncm files here",
        dropCopy: "Choose multiple files at once, or use the button below",
        selectFiles: "Choose files",
        fileInputLabel: "Choose NCM files",
        memoryGuidance:
            "Large files use browser memory while being decrypted. Select fewer files at once if your device is low on memory.",
        filesHeading: "Files",
        clearList: "Clear list",
        emptyState: "Conversion status and download actions will appear here after you choose files.",
        fileListLabel: "Conversion file list",
        disclaimerTitle: "Disclaimer",
        disclaimerText:
            "Use this tool only with audio files you are legally authorized to decrypt, convert, or otherwise process. You are solely responsible for all legal liabilities and other consequences arising from its use; the developers and contributors accept no responsibility.",
        footerPrivacy:
            "All files are processed only in this browser tab and are never sent to a server.",
        builtFrom: "Built from the MIT-licensed",
        builtSuffix: "project",
        viewSource: "View source",
        emptySummary: "No files added",
        statusWaiting: "Waiting",
        statusReading: "Reading",
        statusWorking: "Converting",
        statusSuccess: "Complete",
        statusError: "Failed",
        largeFile:
            "This is a large file. Conversion may require several times its size in free memory; keep this page open.",
        unsupportedFile: "Only files with the .ncm extension are supported.",
        readFailed: "Could not read this file",
        workerFailed: "The converter could not be loaded. Refresh the page and try again.",
        conversionFailed: "Conversion failed",
        damagedFile: "The file may be damaged or may not be a supported NCM file.",
        preview: "Preview {name}",
        download: "Download {format}",
        downloadNamed: "Download {name}",
        summaryWaiting: "{count} waiting",
        summaryWorking: "{count} processing",
        summarySuccess: "{count} complete",
        summaryFailed: "{count} failed",
    },
};

let language = "zh";

export function setLocale(nextLanguage) {
    if (!Object.hasOwn(COPY, nextLanguage)) return false;
    language = nextLanguage;
    return true;
}

export function getLocale() {
    return language;
}

export function t(key, values = {}) {
    let message = COPY[language][key] ?? key;
    for (const [name, value] of Object.entries(values)) {
        message = message.replaceAll(`{${name}}`, String(value));
    }
    return message;
}
