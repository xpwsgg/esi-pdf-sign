import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { open } from "@tauri-apps/plugin-dialog"
import { load, type Store } from "@tauri-apps/plugin-store"

/** Mirror of `CmdSignResult` returned by the Rust backend. */
interface CmdSignResult {
  input: string
  output: string | null
  error: string | null
}

interface ProgressEvent {
  done: number
  total: number
  last: CmdSignResult
}

const TEMPLATE_NAME = "H5P9"
const STORE_FILE = "settings.json"
const SIG_PATH_KEY = "last_signature_path"

const state = {
  pdfPaths: [] as string[],
  signaturePath: null as string | null,
}

let store: Store | null = null
async function initStore(): Promise<void> {
  store = await load(STORE_FILE)
  const last = await store.get<string>(SIG_PATH_KEY)
  if (typeof last === "string" && last.length > 0) {
    state.signaturePath = last
  }
}

async function persistSignaturePath(path: string): Promise<void> {
  if (!store) return
  await store.set(SIG_PATH_KEY, path)
  await store.save()
}

function $<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id)
  if (!el) throw new Error(`Missing element #${id}`)
  return el as T
}

const pickPdfsBtn = $<HTMLButtonElement>("pick-pdfs")
const pickSigBtn = $<HTMLButtonElement>("pick-signature")
const signBtn = $<HTMLButtonElement>("sign")
const pdfSummary = $("pdf-summary")
const sigPath = $("sig-path")
const statusEl = $("status")
const resultsSection = $("results-section")
const resultsList = $<HTMLUListElement>("results")

function basename(p: string): string {
  const norm = p.replace(/\\/g, "/")
  const idx = norm.lastIndexOf("/")
  return idx >= 0 ? norm.slice(idx + 1) : norm
}

function updateSignEnabled() {
  signBtn.disabled =
    state.pdfPaths.length === 0 || state.signaturePath === null
}

function updatePdfSummary() {
  if (state.pdfPaths.length === 0) {
    pdfSummary.textContent = "尚未选择"
  } else if (state.pdfPaths.length === 1) {
    pdfSummary.textContent = basename(state.pdfPaths[0])
  } else {
    pdfSummary.textContent = `已选 ${state.pdfPaths.length} 份 PDF`
  }
}

function updateSigSummary() {
  sigPath.textContent = state.signaturePath
    ? basename(state.signaturePath)
    : "尚未选择"
}

pickPdfsBtn.addEventListener("click", async () => {
  const picked = await open({
    multiple: true,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  })
  if (!picked) return
  state.pdfPaths = Array.isArray(picked) ? picked : [picked]
  updatePdfSummary()
  updateSignEnabled()
})

pickSigBtn.addEventListener("click", async () => {
  const picked = await open({
    multiple: false,
    filters: [{ name: "PNG", extensions: ["png"] }],
  })
  if (!picked || Array.isArray(picked)) return
  state.signaturePath = picked
  updateSigSummary()
  updateSignEnabled()
  void persistSignaturePath(picked)
})

let unlistenProgress: UnlistenFn | null = null

function renderResultItem(r: CmdSignResult) {
  const li = document.createElement("li")
  const icon = document.createElement("span")
  const file = document.createElement("span")
  const detail = document.createElement("span")
  file.className = "file"
  file.textContent = basename(r.input)
  detail.className = "detail"
  if (r.output) {
    icon.className = "icon-ok"
    icon.textContent = "✓"
    detail.textContent = `→ ${basename(r.output)}`
  } else {
    icon.className = "icon-err"
    icon.textContent = "✗"
    detail.textContent = r.error ?? "(unknown error)"
  }
  li.appendChild(icon)
  li.appendChild(file)
  li.appendChild(detail)
  resultsList.appendChild(li)
}

signBtn.addEventListener("click", async () => {
  if (state.signaturePath === null || state.pdfPaths.length === 0) return

  signBtn.disabled = true
  pickPdfsBtn.disabled = true
  pickSigBtn.disabled = true
  resultsList.innerHTML = ""
  resultsSection.removeAttribute("hidden")
  statusEl.textContent = `0/${state.pdfPaths.length}…`

  // subscribe to progress events
  if (unlistenProgress) unlistenProgress()
  unlistenProgress = await listen<ProgressEvent>("sign://progress", (e) => {
    statusEl.textContent = `${e.payload.done}/${e.payload.total}…`
    renderResultItem(e.payload.last)
  })

  try {
    const results: CmdSignResult[] = await invoke("sign_pdfs_cmd", {
      pdfPaths: state.pdfPaths,
      signaturePath: state.signaturePath,
      templateName: TEMPLATE_NAME,
    })
    const okCount = results.filter((r) => r.output).length
    const errCount = results.length - okCount
    statusEl.textContent =
      errCount === 0
        ? `全部完成:${okCount}/${results.length} 成功`
        : `完成:${okCount} 成功 / ${errCount} 失败`
  } catch (err) {
    statusEl.textContent = `调用失败:${String(err)}`
  } finally {
    if (unlistenProgress) {
      unlistenProgress()
      unlistenProgress = null
    }
    pickPdfsBtn.disabled = false
    pickSigBtn.disabled = false
    updateSignEnabled()
  }
})

// initial render
updatePdfSummary()
updateSigSummary()
updateSignEnabled()

// async init: load remembered signature path (does not block UI)
initStore()
  .then(() => {
    updateSigSummary()
    updateSignEnabled()
  })
  .catch((err) => {
    console.error("failed to init store:", err)
  })
