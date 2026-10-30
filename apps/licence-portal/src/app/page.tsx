import { redirect } from "next/navigation";
import { surface } from "@/lib/env";

export default function Root() {
  redirect(surface() === "checkout" ? "/buy" : "/dashboard");
}
