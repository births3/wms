/**
 * @wms/ui Tailwind preset
 *
 * 包含 wms 全局设计 token：
 * - shadcn 标准颜色变量（hsl(var(--xxx))）
 * - WMS 业务色（wms-warning / wms-success / wms-cold）
 * - 字体栈、圆角、动画
 *
 * 用法（消费方 tailwind.config.js）：
 *   presets: [require('@wms/ui/tailwind-preset')],
 *   content: [
 *     './src/**\/*.{ts,tsx}',
 *     '../../packages/ui/src/**\/*.{ts,tsx}',  // 必须扫源码以生成 ui 包内的 class
 *   ],
 *
 * CSS 变量定义在 packages/ui/src/styles/globals.css，消费方 import 一次即可。
 */
module.exports = {
  darkMode: ["class"],
  theme: {
    container: { center: true, padding: "2rem", screens: { "2xl": "1400px" } },
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        // WMS 业务色（设计 token）
        "wms-warning": "hsl(var(--wms-warning))",
        "wms-success": "hsl(var(--wms-success))",
        "wms-cold": "hsl(var(--wms-cold))",
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"PingFang SC"',
          '"Microsoft YaHei"',
          "sans-serif",
        ],
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};
