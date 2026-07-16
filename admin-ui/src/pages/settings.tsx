import { Languages, Monitor, Moon, PaintBucket, Settings, Sun } from "lucide-react";
import type { ColorTheme, PageContext } from "../types";
import { PageFrame, Panel } from "../components/common";

const themeOptions: Array<{ value: ColorTheme; key: "systemTheme" | "lightTheme" | "darkTheme"; icon: typeof Monitor }> = [
  { value: "system", key: "systemTheme", icon: Monitor },
  { value: "light", key: "lightTheme", icon: Sun },
  { value: "dark", key: "darkTheme", icon: Moon }
];

export function SettingsPage({ t, language, setLanguage, colorTheme, setColorTheme }: PageContext) {
  return (
    <PageFrame title={t("settings")} description={t("settingsPageHint")}>
      <div className="settings-stack">
        <Panel title={t("generalSettings")} icon={Settings} className="span-12">
          <div className="settings-section-list">
            <div className="settings-row-item">
              <div className="settings-row-icon">
                <Languages aria-hidden="true" size={18} />
              </div>
              <div className="settings-row-copy">
                <strong>{t("interfaceLanguage")}</strong>
                <span>{t("interfaceLanguageHint")}</span>
              </div>
              <label className="settings-select-wrap">
                <select aria-label={t("interfaceLanguage")} value={language} onChange={(event) => setLanguage(event.target.value === "en" ? "en" : "zh")}>
                  <option value="zh">中文</option>
                  <option value="en">English</option>
                </select>
              </label>
            </div>

            <div className="settings-row-item">
              <div className="settings-row-icon">
                <PaintBucket aria-hidden="true" size={18} />
              </div>
              <div className="settings-row-copy">
                <strong>{t("interfaceTheme")}</strong>
                <span>{t("interfaceThemeHint")}</span>
              </div>
              <div className="settings-theme-control" aria-label={t("interfaceTheme")} role="group">
                {themeOptions.map((option) => {
                  const Icon = option.icon;
                  const selected = colorTheme === option.value;
                  return (
                    <button
                      aria-label={t(option.key)}
                      aria-pressed={selected}
                      className={selected ? "is-active" : ""}
                      key={option.value}
                      title={t(option.key)}
                      type="button"
                      onClick={() => setColorTheme(option.value)}
                    >
                      <Icon aria-hidden="true" size={17} />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        </Panel>
      </div>
    </PageFrame>
  );
}
