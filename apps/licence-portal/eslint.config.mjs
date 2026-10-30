import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const config = [
  ...nextVitals,
  ...nextTs,
  { ignores: [".next/**", "node_modules/**", "next-env.d.ts"] },
  {
    rules: {
      // Secrets must never reach the browser: forbid the public env prefix for anything
      // except the explicitly public variables documented in .env.example.
      "no-restricted-syntax": [
        "error",
        {
          selector: "MemberExpression[object.object.name='process'][object.property.name='env'][property.name=/^NEXT_PUBLIC_/]",
          message: "No NEXT_PUBLIC_ variables: the portal has no browser-side configuration.",
        },
      ],
    },
  },
];

export default config;
