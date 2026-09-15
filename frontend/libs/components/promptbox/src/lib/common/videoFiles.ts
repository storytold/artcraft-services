import { isAudioFile } from "./audioFiles";

const MP4_FILE_ACCEPT = "video/mp4,.mp4";
const QUICKTIME_FILE_ACCEPT = "video/quicktime,.mov,.qt";

export function getVideoFileAccept(allowQuicktime: boolean): string {
  return allowQuicktime
    ? `${MP4_FILE_ACCEPT},${QUICKTIME_FILE_ACCEPT}`
    : MP4_FILE_ACCEPT;
}

export function getVideoFileTypeError(allowQuicktime: boolean): string {
  return allowQuicktime
    ? "Please choose an MP4 or QuickTime (MOV) video."
    : "Please choose an MP4 video.";
}

// File pickers and drops can supply an empty or generic MIME type for MOV.
// The server verifies the container from the file bytes before accepting it.
export function isVideoFile(file: File, allowQuicktime: boolean): boolean {
  if (isAudioFile(file)) return false;
  const extension = file.name.split(".").pop()?.toLowerCase();
  if (file.type === "video/quicktime" || extension === "mov" || extension === "qt") {
    return allowQuicktime;
  }
  return file.type === "video/mp4" || extension === "mp4";
}
