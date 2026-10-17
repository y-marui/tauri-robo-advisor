import { icloudDataFolder, setDataFolder } from "../api.ts";
import { h } from "../dom.ts";
import { t } from "../i18n.ts";
import type { Context, Panel } from "../panel.ts";
import type { DataInfo, PortfolioView } from "../types.ts";

// Where the data file lives. The folder can be one that the OS already syncs
// between computers (iCloud Drive); the app itself sends nothing anywhere.

const FOLDER_HAS_DATA = "FOLDER_HAS_DATA";

/** Messages about the folder that the user should not miss; `null` when there are none. */
export function dataNotes(data: DataInfo): HTMLElement | null {
  const notes: string[] = [];
  if (data.waiting_for_download) {
    notes.push(
      t("The data file is still being downloaded from the synced folder. Wait a moment, then come back to this window."),
    );
  }
  if (data.conflict_copies.length > 0) {
    notes.push(
      t("Other copies of the data file were found: {names}. Two computers saved at the same time; compare them and remove the ones you do not need.", {
        names: data.conflict_copies.join(", "),
      }),
    );
  }
  if (data.warning) {
    notes.push(t("A newer data file could not be used: {reason}", { reason: data.warning }));
  }
  if (notes.length === 0) {
    return null;
  }
  return h("ul", { class: "issues", role: "status" }, ...notes.map((n) => h("li", {}, n)));
}

export function dataFolderPanel(ctx: Context): Panel {
  const current = h("p", { class: "note" });
  const folderInput = h("input", {
    type: "text",
    autocomplete: "off",
    "aria-label": t("Data folder"),
    placeholder: t("Full path of a folder"),
  });
  const useButton = h("button", { type: "button" }, t("Use this folder"));
  const icloudButton = h("button", { type: "button" }, t("Use the iCloud Drive folder"));
  const localButton = h("button", { type: "button" }, t("Use the local folder"));
  const confirmBox = h("div", { class: "add-row", hidden: true });
  icloudButton.hidden = true;

  let shown: PortfolioView | null = null;
  let icloud: string | null = null;

  async function move(folder: string | null, useExisting: boolean): Promise<void> {
    for (const button of [useButton, icloudButton, localButton]) {
      button.disabled = true;
    }
    try {
      const next = await setDataFolder(folder, useExisting);
      confirmBox.hidden = true;
      ctx.setView(next, t("The data folder was changed."));
    } catch (error) {
      if (String(error) === FOLDER_HAS_DATA) {
        askToUseExisting(folder);
      } else {
        confirmBox.hidden = true;
        ctx.fail(t("Could not change the data folder: {reason}", { reason: String(error) }));
      }
    } finally {
      for (const button of [useButton, icloudButton, localButton]) {
        button.disabled = false;
      }
    }
  }

  function askToUseExisting(folder: string | null): void {
    const yes = h("button", { type: "button" }, t("Use the data in that folder"));
    const no = h("button", { type: "button" }, t("Cancel"));
    yes.addEventListener("click", () => void move(folder, true));
    no.addEventListener("click", () => {
      confirmBox.hidden = true;
    });
    confirmBox.replaceChildren(
      h(
        "span",
        {},
        t("That folder already has data. Use it as it is? The data on this computer stays where it is, as a backup."),
      ),
      yes,
      no,
    );
    confirmBox.hidden = false;
  }

  useButton.addEventListener("click", () => {
    const folder = folderInput.value.trim();
    if (folder === "") {
      ctx.fail(t("Enter the full path of a folder."));
      return;
    }
    void move(folder, false);
  });
  icloudButton.addEventListener("click", () => {
    if (icloud) {
      void move(icloud, false);
    }
  });
  localButton.addEventListener("click", () => void move(null, false));

  icloudDataFolder()
    .then((folder) => {
      icloud = folder;
      icloudButton.hidden = folder === null;
    })
    .catch(() => {
      icloudButton.hidden = true;
    });

  const el = h(
    "section",
    { class: "card data-folder" },
    h("h2", {}, t("Data folder")),
    current,
    h("div", { class: "add-row" }, folderInput, useButton, icloudButton, localButton),
    confirmBox,
    h(
      "p",
      { class: "note" },
      t("A folder that your computer syncs (for example iCloud Drive) shares the data between your computers. The app sends nothing itself. The file holds your holdings, so choose a place you trust."),
    ),
  );

  return {
    el,
    render(view) {
      shown = view;
      current.textContent = view.data.is_local
        ? t("The data is kept on this computer only: {folder}", { folder: view.data.folder })
        : t("The data is kept in: {folder}", { folder: view.data.folder });
      localButton.hidden = shown.data.is_local;
    },
  };
}
