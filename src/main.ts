import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { open } from "@tauri-apps/plugin-dialog"
import { revealItemInDir } from "@tauri-apps/plugin-opener"
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

/** Mirror of `CmdWorktime*` returned by the Rust backend. */
interface CmdWorktimeRow {
  part_number: string
  description: string
  qty: number
}

interface CmdWorktime {
  input: string
  rows: CmdWorktimeRow[] | null
  total: number | null
  error: string | null
}

interface CmdWorktimeBatch {
  items: CmdWorktime[]
  grand_total: number
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

async function clearKey(key: string): Promise<void> {
  if (!store) return
  await store.delete(key)
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
const clearEngineerSigBtn = $<HTMLButtonElement>("clear-engineer-signature")
const clearCustomerSigBtn = $<HTMLButtonElement>("clear-customer-signature")
const signBtn = $<HTMLButtonElement>("sign")
const pdfSummary = $("pdf-summary")
const engineerSigPath = $("engineer-sig-path")
const customerSigPath = $("customer-sig-path")
const statusEl = $("status")
const resultsSection = $("results-section")
const resultsList = $<HTMLUListElement>("results")
const worktimeSection = $("worktime-section")
const worktimeGrand = $("worktime-grand")
const worktimeBody = $("worktime-body")

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
  clearEngineerSigBtn.hidden = state.engineerSignaturePath === null
  clearCustomerSigBtn.hidden = state.customerSignaturePath === null
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
  void loadWorktimes()
})

function renderWorktime(batch: CmdWorktimeBatch) {
  worktimeBody.innerHTML = ""
  worktimeGrand.innerHTML = ""
  worktimeGrand.setAttribute("hidden", "")
  if (batch.items.length === 0) {
    worktimeSection.setAttribute("hidden", "")
    return
  }

  for (const item of batch.items) {
    const block = document.createElement("div")
    block.className = "wt-item"

    const head = document.createElement("div")
    head.className = "wt-item-head"
    const name = document.createElement("span")
    name.className = "wt-file"
    name.textContent = basename(item.input)
    head.appendChild(name)

    if (item.error) {
      const err = document.createElement("span")
      err.className = "wt-error"
      err.textContent = item.error
      head.appendChild(err)
      block.appendChild(head)
    } else {
      const sub = document.createElement("span")
      sub.className = "wt-subtotal"
      sub.textContent = `${item.total ?? 0} 小时`
      head.appendChild(sub)
      block.appendChild(head)

      const rows = item.rows ?? []
      if (rows.length > 0) {
        const ul = document.createElement("ul")
        ul.className = "wt-rows"
        for (const r of rows) {
          const li = document.createElement("li")
          const desc = document.createElement("span")
          desc.className = "wt-desc"
          desc.textContent = r.description || r.part_number || "(未命名)"
          const qty = document.createElement("span")
          qty.className = "wt-qty"
          qty.textContent = String(r.qty)
          li.appendChild(desc)
          li.appendChild(qty)
          ul.appendChild(li)
        }
        block.appendChild(ul)
      }
    }
    worktimeBody.appendChild(block)
  }

  const label = document.createElement("span")
  label.textContent = "合计工时"
  const val = document.createElement("span")
  val.className = "wt-grand-val"
  val.textContent = `${batch.grand_total} 小时`
  worktimeGrand.appendChild(label)
  worktimeGrand.appendChild(val)
  worktimeGrand.removeAttribute("hidden")

  worktimeSection.removeAttribute("hidden")
}

// Monotonic request id: each PDF (re)selection bumps this. An in-flight
// extraction only renders if its id still matches — so a slow earlier request
// can't overwrite the panel with stale data after a faster later one.
let worktimeReqId = 0

async function loadWorktimes(): Promise<void> {
  const reqId = ++worktimeReqId
  if (state.pdfPaths.length === 0) {
    worktimeSection.setAttribute("hidden", "")
    return
  }
  worktimeBody.innerHTML = ""
  worktimeSection.removeAttribute("hidden")
  const loading = document.createElement("div")
  loading.className = "wt-loading"
  loading.textContent = "正在统计工时…"
  worktimeBody.appendChild(loading)

  try {
    const batch: CmdWorktimeBatch = await invoke("extract_worktimes_cmd", {
      pdfPaths: state.pdfPaths,
    })
    if (reqId !== worktimeReqId) return // superseded by a newer selection
    renderWorktime(batch)
  } catch (err) {
    if (reqId !== worktimeReqId) return // superseded by a newer selection
    worktimeBody.innerHTML = ""
    const e = document.createElement("div")
    e.className = "wt-error"
    e.textContent = `工时统计失败:${String(err)}`
    worktimeBody.appendChild(e)
  }
}

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

clearEngineerSigBtn.addEventListener("click", () => {
  state.engineerSignaturePath = null
  updateSigSummaries()
  updateSignEnabled()
  void clearKey(ENGINEER_SIG_KEY)
})

clearCustomerSigBtn.addEventListener("click", () => {
  state.customerSignaturePath = null
  updateSigSummaries()
  updateSignEnabled()
  void clearKey(CUSTOMER_SIG_KEY)
})

let unlistenProgress: UnlistenFn | null = null

function renderResultItem(r: CmdSignResult) {
  const li = document.createElement("li")
  const icon = document.createElement("span")
  const file = document.createElement("span")
  file.className = "file"
  file.textContent = basename(r.input)
  li.appendChild(icon)
  li.appendChild(file)

  if (r.output) {
    icon.className = "icon-ok"
    icon.textContent = "✓"
    const reveal = document.createElement("button")
    reveal.type = "button"
    reveal.className = "btn btn-ghost reveal"
    reveal.textContent = "在文件夹中显示"
    reveal.title = r.output
    const output = r.output
    reveal.addEventListener("click", () => {
      revealItemInDir(output).catch((err) => {
        console.error("revealItemInDir failed:", err)
      })
    })
    li.appendChild(reveal)
  } else {
    icon.className = "icon-err"
    icon.textContent = "✗"
    const detail = document.createElement("span")
    detail.className = "detail"
    detail.textContent = r.error ?? "(unknown error)"
    li.appendChild(detail)
  }

  resultsList.appendChild(li)
}

signBtn.addEventListener("click", async () => {
  if (state.engineerSignaturePath === null || state.pdfPaths.length === 0) return

  signBtn.disabled = true
  pickPdfsBtn.disabled = true
  pickEngineerSigBtn.disabled = true
  pickCustomerSigBtn.disabled = true
  clearEngineerSigBtn.disabled = true
  clearCustomerSigBtn.disabled = true
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
    clearEngineerSigBtn.disabled = false
    clearCustomerSigBtn.disabled = false
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
