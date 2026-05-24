import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Rect = { x: number; y: number; w: number; h: number };
type Status = "idle" | "ready" | "selecting" | "processing";

export default function App() {
  const [rect, setRect] = useState<Rect | null>(null);
  const [status, setStatus] = useState<Status>("idle");
  const [scaleX, setScaleX] = useState(1);
  const [scaleY, setScaleY] = useState(1);
  const startRef = useRef<{ x: number; y: number } | null>(null);
  const [copyFormat, setCopyFormat] = useState<"plain" | "markdown" | "html">(
    () => (localStorage.getItem("copy-format") as "plain" | "markdown" | "html") || "plain"
  );

  const closeOverlay = useCallback(async () => {
    setRect(null);
    setStatus("idle");
    startRef.current = null;
    await invoke("close_overlay");
  }, []);

  // Listen for the "capture-ready" event fired by Rust after a successful screen grab
  useEffect(() => {
    const unlistenPromise = listen("capture-ready", async () => {
      try {
        const [w, h] = await invoke<[number, number]>("get_capture_dimensions");
        setScaleX(w / window.innerWidth);
        setScaleY(h / window.innerHeight);
        setRect(null);
        setStatus("ready");
      } catch (e) {
        console.error("capture-ready handler failed:", e);
      }
    });
    return () => {
      unlistenPromise.then((f) => f());
    };
  }, []);

  // Esc to cancel
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        closeOverlay();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closeOverlay]);

  const onMouseDown = (e: React.MouseEvent) => {
    if (status === "processing") return;
    startRef.current = { x: e.clientX, y: e.clientY };
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
    setStatus("selecting");
  };

  const onMouseMove = (e: React.MouseEvent) => {
    if (status !== "selecting" || !startRef.current) return;
    const s = startRef.current;
    setRect({
      x: Math.min(s.x, e.clientX),
      y: Math.min(s.y, e.clientY),
      w: Math.abs(e.clientX - s.x),
      h: Math.abs(e.clientY - s.y),
    });
  };

  const onMouseUp = async () => {
    if (status !== "selecting" || !rect) return;

    if (rect.w < 5 || rect.h < 5) {
      // Treat as accidental click
      closeOverlay();
      return;
    }

    setStatus("processing");
    try {
      await invoke("process_selection", {
        x: Math.round(rect.x * scaleX),
        y: Math.round(rect.y * scaleY),
        w: Math.round(rect.w * scaleX),
        h: Math.round(rect.h * scaleY),
        format: copyFormat,
      });
    } catch (e) {
      console.error("OCR failed:", e);
    } finally {
      // Rust hides the window itself once OCR completes, but reset state regardless
      setRect(null);
      setStatus("idle");
      startRef.current = null;
    }
  };

  const showSelection = rect && (status === "selecting" || status === "processing");
  const dims = rect ? `${Math.round(rect.w)} × ${Math.round(rect.h)}` : "";

  return (
    <div
      className="overlay"
      data-status={status}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      {/* Crosshair guides — only visible when idle/ready */}
      {status === "ready" && <Crosshair />}

      {showSelection && (
        <>
          {/* Four masks around the selection to "punch out" the dimming */}
          <div className="mask" style={{ left: 0, top: 0, right: 0, height: rect!.y }} />
          <div
            className="mask"
            style={{ left: 0, top: rect!.y, width: rect!.x, height: rect!.h }}
          />
          <div
            className="mask"
            style={{
              left: rect!.x + rect!.w,
              top: rect!.y,
              right: 0,
              height: rect!.h,
            }}
          />
          <div
            className="mask"
            style={{ left: 0, top: rect!.y + rect!.h, right: 0, bottom: 0 }}
          />

          <div
            className="selection"
            style={{
              left: rect!.x,
              top: rect!.y,
              width: rect!.w,
              height: rect!.h,
            }}
          >
            <span className="dim-badge">{dims}</span>
          </div>
        </>
      )}

      {status !== "processing" && (
        <div className="format-pill" onMouseDown={(e) => e.stopPropagation()}>
          {(["plain", "markdown", "html"] as const).map((f) => (
            <button
              key={f}
              className={`format-btn${copyFormat === f ? " active" : ""}`}
              onClick={() => {
                setCopyFormat(f);
                localStorage.setItem("copy-format", f);
              }}
            >
              {f === "plain" ? "Plain" : f === "markdown" ? "Markdown" : "HTML"}
            </button>
          ))}
        </div>
      )}

      <div className="hint">
        {status === "processing" ? (
          <>
            <span className="spinner" />
            Reading text…
          </>
        ) : (
          <>Drag to capture · <kbd>Esc</kbd> to cancel</>
        )}
      </div>
    </div>
  );
}

function Crosshair() {
  // Track cursor for full-screen crosshair lines (subtle, hairline)
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    const onMove = (e: MouseEvent) => setPos({ x: e.clientX, y: e.clientY });
    window.addEventListener("mousemove", onMove);
    return () => window.removeEventListener("mousemove", onMove);
  }, []);
  if (!pos) return null;
  return (
    <>
      <div className="crosshair-h" style={{ top: pos.y }} />
      <div className="crosshair-v" style={{ left: pos.x }} />
    </>
  );
}
