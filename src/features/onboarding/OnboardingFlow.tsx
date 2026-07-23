import { zodResolver } from "@hookform/resolvers/zod"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { ArrowRight, LockKeyhole, Sparkles } from "lucide-react"
import { useForm } from "react-hook-form"
import { useNavigate } from "react-router-dom"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { queryKeys } from "@/lib/query-keys"
import { invokeValidated, isTauriRuntime } from "@/lib/tauri"
import { founderInputSchema, founderProfileSchema, type FounderInput } from "@/schemas/founder"

export function OnboardingFlow() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const form = useForm<FounderInput>({
    resolver: zodResolver(founderInputSchema),
    defaultValues: {
      name: "",
      productName: "",
      offer: "",
      expertise: "",
      goals: ["Build qualified founder relationships"],
      boundaries: ["No spam or invented claims"],
    },
  })
  const save = useMutation({
    mutationFn: async (input: FounderInput) => {
      if (!isTauriRuntime())
        return founderProfileSchema.parse({
          ...input,
          id: crypto.randomUUID(),
          onboardingCompleted: true,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        })
      return invokeValidated("save_founder_profile", { input }, founderInputSchema, founderProfileSchema)
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.bootstrap })
      void navigate("/")
    },
  })

  return (
    <div className="onboarding-layout">
      <aside className="onboarding-aside">
        <span className="brand-mark large">
          <Sparkles />
        </span>
        <p className="eyebrow">Founder baseline</p>
        <h1>Give the lab a point of view to protect.</h1>
        <p>
          This becomes the local context Codex or Claude uses to draft—not a public profile and not model
          training.
        </p>
        <div className="privacy-note">
          <LockKeyhole size={18} />
          <span>
            <strong>Stored locally</strong>
            <small>You can edit, export, or reset this memory.</small>
          </span>
        </div>
      </aside>
      <form
        className="onboarding-form"
        onSubmit={(event) => void form.handleSubmit((value) => save.mutate(value))(event)}
      >
        <div className="form-heading">
          <span>01</span>
          <div>
            <p className="eyebrow">Your operating context</p>
            <h2>What should the system understand?</h2>
          </div>
        </div>
        <div className="field-grid two">
          <Field label="Your name" error={form.formState.errors.name?.message}>
            <Input autoFocus {...form.register("name")} placeholder="Duc" />
          </Field>
          <Field label="Product or project" error={form.formState.errors.productName?.message}>
            <Input {...form.register("productName")} placeholder="Your startup" />
          </Field>
        </div>
        <Field
          label="What do you offer?"
          hint="Describe the transformation, not just the feature."
          error={form.formState.errors.offer?.message}
        >
          <Textarea rows={4} {...form.register("offer")} placeholder="I help…" />
        </Field>
        <Field
          label="What have you earned the right to talk about?"
          hint="Experience, hard-won lessons, and strong opinions."
          error={form.formState.errors.expertise?.message}
        >
          <Textarea rows={5} {...form.register("expertise")} placeholder="I have spent…" />
        </Field>
        <div className="field-grid two">
          <Field label="Primary goal">
            <Input {...form.register("goals.0")} />
          </Field>
          <Field label="Non-negotiable boundary">
            <Input {...form.register("boundaries.0")} />
          </Field>
        </div>
        {save.isError && (
          <div className="inline-error">
            <strong>Could not save</strong>
            <span>{save.error.message}</span>
          </div>
        )}
        <div className="form-actions">
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? "Saving locally…" : "Save founder baseline"}
            <ArrowRight size={16} />
          </Button>
        </div>
      </form>
    </div>
  )
}

function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string
  hint?: string
  error?: string
  children: React.ReactNode
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {hint && <small>{hint}</small>}
      {children}
      {error && <em>{error}</em>}
    </label>
  )
}
