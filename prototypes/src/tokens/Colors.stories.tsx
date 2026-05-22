import type { Meta, StoryObj } from "@storybook/react";
import { colors } from "../tokens";

function TokensDemo() {
  return (
    <div style={{ fontFamily: "system-ui", padding: 24 }}>
      <h2>WMS Design Tokens</h2>
      <div style={{ display: "flex", gap: 8, marginTop: 16 }}>
        {Object.entries(colors)
          .filter(([, v]) => typeof v === "string")
          .map(([name, value]) => (
            <div key={name} style={{ textAlign: "center" }}>
              <div
                style={{
                  width: 48,
                  height: 48,
                  borderRadius: 8,
                  background: value as string,
                }}
              />
              <small>{name}</small>
            </div>
          ))}
      </div>
    </div>
  );
}

const meta: Meta = { title: "Tokens/Colors", component: TokensDemo };
export default meta;

export const Default: StoryObj = {};
