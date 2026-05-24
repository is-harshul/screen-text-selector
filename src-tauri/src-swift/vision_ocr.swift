import Foundation
import Vision

// Minimum deployment: macOS 11.0

// Called from Rust via extern "C". Caller owns the returned pointer and must free it.
@_cdecl("vision_ocr_from_png_bytes")
public func visionOcrFromPngBytes(
    ptr: UnsafePointer<UInt8>,
    len: Int,
    outPtr: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>
) -> Int {
    let data = Data(bytes: ptr, count: len)
    guard
        let src = CGImageSourceCreateWithData(data as CFData, nil),
        let cgImage = CGImageSourceCreateImageAtIndex(src, 0, nil)
    else {
        outPtr.pointee = nil
        return 0
    }

    var lines: [String] = []
    let semaphore = DispatchSemaphore(value: 0)

    let request = VNRecognizeTextRequest { req, _ in
        defer { semaphore.signal() }
        guard let obs = req.results as? [VNRecognizedTextObservation] else { return }
        lines = obs.compactMap { $0.topCandidates(1).first?.string }
    }
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    // recognizes emoji in rendered glyphs
    request.recognitionLanguages = ["en-US"]

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
    try? handler.perform([request])
    semaphore.wait()

    let joined = lines.joined(separator: "\n")
    // strdup allocates — Rust must call vision_ocr_free on the pointer
    outPtr.pointee = strdup(joined)
    return joined.utf8.count
}

@_cdecl("vision_ocr_free")
public func visionOcrFree(ptr: UnsafeMutablePointer<CChar>?) {
    guard let p = ptr else { return }
    free(p)
}
