import { useEffect, useState } from "react";
import { errorMessage, ipc, type SettingsInfo } from "@/lib/ipc";
import { useAppStore } from "@/stores/app";
import { Field, inputClass, primaryButtonClass } from "./fields";

export function GeneralSettings() {
  const [info, setInfo] = useState<SettingsInfo | null>(null);
  const [editor, setEditor] = useState("");
  const [notify, setNotify] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const adopt = (next: SettingsInfo) => {
    setInfo(next);
    setEditor(next.editorCommand ?? "");
    setNotify(next.notifyWhenQuiet);
    useAppStore.setState({ notifyWhenQuiet: next.notifyWhenQuiet });
  };
  useEffect(() => {
    ipc.settingsGet().then(adopt, (reason) => setError(errorMessage(reason)));
  }, []);

  if (!info) return <p className="p-5 text-ink-faint">{error ?? "Loading…"}</p>;
  const dirty = editor.trim() !== (info.editorCommand ?? "") || notify !== info.notifyWhenQuiet;

  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    try {
      adopt(await ipc.settingsSaveGeneral(editor.trim() || null, notify));
      setSaved(true);
    } catch (reason) {
      setError(errorMessage(reason));
    }
  };

  return (
    <form aria-label="General settings" onSubmit={save} className="grid max-w-2xl gap-4 p-5">
      <Field
        label="Editor command"
        hint={
          <>
            Used by “Open in editor”. It is given the workspace folder, then the file — for example{" "}
            <code>code</code>, <code>cursor</code> or <code>zed</code>. Leave empty to try the
            common editors in turn.
          </>
        }
      >
        <input
          value={editor}
          placeholder="auto-detect"
          spellCheck={false}
          onChange={(e) => {
            setEditor(e.target.value);
            setSaved(false);
          }}
          className={`${inputClass} w-72 font-mono text-[12px]`}
        />
      </Field>
      <label className="flex items-start gap-2">
        <input
          type="checkbox"
          checked={notify}
          onChange={(e) => {
            setNotify(e.target.checked);
            setSaved(false);
          }}
          className="mt-0.5 accent-(--color-accent)"
        />
        <span>
          Notify me when an agent finishes
          <span className="block text-ink-faint">
            A desktop notification when an agent that worked for a while goes quiet and Switchyard
            is not the window you are looking at.
          </span>
        </span>
      </label>
      {error && (
        <p role="alert" className="text-red-400 select-text">
          {error}
        </p>
      )}
      <div className="flex items-center gap-3">
        <button type="submit" disabled={!dirty} className={primaryButtonClass}>
          Save
        </button>
        {saved && !dirty && <span className="text-ink-faint">Saved.</span>}
      </div>
    </form>
  );
}
