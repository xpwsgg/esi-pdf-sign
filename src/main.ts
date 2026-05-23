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
const ENGINEER_SIG_KEY = "last_engineer_signature_path"
const CUSTOMER_SIG_KEY = "last_customer_signature_path"

const state = {
  pdfPaths: [] as string[],
  engineerSignaturePath: null as string | null,
  customerSignaturePath: null as string | null,
}

let store: Store | null = null
async function initStore(): Promise<void> {
  store = await load(STORE_FILE)
  const e = await store.get<string>(ENGINEER_SIG_KEY)
  if (typeof e === "string" && e.length > 0) state.engineerSignaturePath = e
  const c = await store.get<string>(CUSTOMER_SIG_KEY)
  if (typeof c === "string" && c.length > 0) state.customerSignaturePath = c
}

async function persistKey(key: string, value: string): Promise<void> {
  if (!store) return
  await store.set(key, value)
  await store.save()
}

function $<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id)
  if (!el) throw new Error(`Missing element #${id}`)
  return el as T
}

const pickPdfsBtn = $<HTMLButtonElement>("pick-pdfs")
const pickEngineerSigBtn = $<HTMLButtonElement>("pick-engineer-signature")
const pickCustomerSigBtn = $<HTMLButtonElement>("pick-customer-signature")
const signBtn = $<HTMLButtonElement>("sign")
const pdfSummary = $("pdf-summary")
const engineerSigPath = $("engineer-sig-path")
const customerSigPath = $("customer-sig-path")
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
    state.pdfPaths.length === 0 || state.engineerSignaturePath === null
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

function updateSigSummaries() {
  engineerSigPath.textContent = state.engineerSignaturePath
    ? basename(state.engineerSignaturePath)
    : "尚未选择"
  customerSigPath.textContent = state.customerSignaturePath
    ? basename(state.customerSignaturePath)
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

async function pickPng(): Promise<string | null> {
  const picked = await open({
    multiple: false,
    filters: [{ name: "PNG", extensions: ["png"] }],
  })
  if (!picked || Array.isArray(picked)) return null
  return picked
}

pickEngineerSigBtn.addEventListener("click", async () => {
  const p = await pickPng()
  if (p === null) return
  state.engineerSignaturePath = p
  updateSigSummaries()
  updateSignEnabled()
  void persistKey(ENGINEER_SIG_KEY, p)
})

pickCustomerSigBtn.addEventListener("click", async () => {
  const p = await pickPng()
  if (p === null) return
  state.customerSignaturePath = p
  updateSigSummaries()
  updateSignEnabled()
  void persistKey(CUSTOMER_SIG_KEY, p)
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
  if (state.engineerSignaturePath === null || state.pdfPaths.length === 0) return

  signBtn.disabled = true
  pickPdfsBtn.disabled = true
  pickEngineerSigBtn.disabled = true
  pickCustomerSigBtn.disabled = true
  resultsList.innerHTML = ""
  resultsSection.removeAttribute("hidden")
  statusEl.textContent = `0/${state.pdfPaths.length}…`

  if (unlistenProgress) unlistenProgress()
  unlistenProgress = await listen<ProgressEvent>("sign://progress", (e) => {
    statusEl.textContent = `${e.payload.done}/${e.payload.total}…`
    renderResultItem(e.payload.last)
  })

  const signaturePaths: Record<string, string> = {
    engineer: state.engineerSignaturePath,
  }
  if (state.customerSignaturePath) {
    signaturePaths.customer = state.customerSignaturePath
  }

  try {
    const results: CmdSignResult[] = await invoke("sign_pdfs_cmd", {
      pdfPaths: state.pdfPaths,
      signaturePaths,
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
    pickEngineerSigBtn.disabled = false
    pickCustomerSigBtn.disabled = false
    updateSignEnabled()
  }
})

updatePdfSummary()
updateSigSummaries()
updateSignEnabled()

initStore()
  .then(() => {
    updateSigSummaries()
    updateSignEnabled()
  })
  .catch((err) => {
    console.error("failed to init store:", err)
  })
