import { defineOverridesPreferences } from '@vben/preferences';

/**
 * @description 项目配置文件
 * 只需要覆盖项目中的一部分配置，不需要的配置不用覆盖，会自动使用默认配置
 * !!! 更改配置后请清空缓存，否则可能不生效
 */
export const overridesPreferences = defineOverridesPreferences({
  // overrides
  app: {
    name: import.meta.env.VITE_APP_TITLE,
    defaultHomePath: "/dashboard/traffic"

  },

  logo: {
    source: "/logo.png",
  },
  copyright: {
    companyName: "Aigw.net",
    /** 版权公司名链接 */
    companySiteLink: "https://aigw.net",
    /** 版权日期 */
    date: new Date().getFullYear().toString(),
    /** 版权是否可见 */
    enable: true,
    /** 备案号 */
    icp: "",
    /** 备案号链接 */
    icpLink: "",
    /** 设置面板是否显示*/
    settingShow: true,
  },
});
