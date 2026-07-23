import { zodResolver } from "@hookform/resolvers/zod"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { ArrowRight, Check, Globe2, LockKeyhole, MessageSquareText, Sparkles } from "lucide-react"
import { cloneElement, useId, type ReactElement } from "react"
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
      websiteUrl: "",
      offer: "",
      idealCustomer: "",
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
          websiteUrl: input.websiteUrl || null,
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
    <div className="onboarding-shell">
      <header className="onboarding-intro">
        <div className="onboarding-intro-topline">
          <span className="brand-mark onboarding-mark" aria-hidden="true">
            <Sparkles size={18} />
          </span>
          <span className="onboarding-draft-status">
            <span aria-hidden="true" />
            Starting profile · editable
          </span>
        </div>
        <p className="eyebrow">Welcome to Goalbar</p>
        <h1>Start with what you already know.</h1>
        <p>
          Paste a landing page or describe what you are building. This is a starting point, not a test—
          Goalbar can refine it as you gather real evidence.
        </p>
        <div className="onboarding-learning-loop" aria-label="How your profile improves">
          <span>
            <Check size={13} aria-hidden="true" /> Start now
          </span>
          <i aria-hidden="true" />
          <span>Learn from evidence</span>
          <i aria-hidden="true" />
          <span>Refine over time</span>
        </div>
      </header>
      <form
        className="onboarding-card"
        onSubmit={(event) => void form.handleSubmit((value) => save.mutate(value))(event)}
      >
        <section className="onboarding-section" aria-labelledby="business-context-heading">
          <div className="onboarding-section-heading">
            <span>1</span>
            <div>
              <p className="eyebrow">Your business</p>
              <h2 id="business-context-heading">Give us something to start from</h2>
            </div>
          </div>
          <Field
            label="Paste your landing page"
            hint="We will keep the URL as part of your local business context."
            error={form.formState.errors.websiteUrl?.message}
            icon={<Globe2 size={16} />}
          >
            <Input
              autoFocus
              type="url"
              inputMode="url"
              autoComplete="url"
              {...form.register("websiteUrl")}
              placeholder="https://yourcompany.com"
            />
          </Field>
          <div className="onboarding-or" aria-hidden="true">
            <span>or</span>
          </div>
          <Field
            label="Describe it in your own words"
            hint="What are you building, who does it help, and why does it matter?"
            error={form.formState.errors.offer?.message}
            icon={<MessageSquareText size={16} />}
          >
            <Textarea
              rows={4}
              {...form.register("offer")}
              placeholder="I’m building a local-first growth tool that helps solo founders…"
            />
          </Field>
        </section>

        <section className="onboarding-section" aria-labelledby="customer-context-heading">
          <div className="onboarding-section-heading">
            <span>2</span>
            <div>
              <p className="eyebrow">Your ideal customer</p>
              <h2 id="customer-context-heading">Who do you most want to help?</h2>
            </div>
          </div>
          <Field
            label="Describe your ICP"
            hint="A rough answer is enough. Include their role, situation, and the problem they feel."
            error={form.formState.errors.idealCustomer?.message}
          >
            <Textarea
              rows={4}
              {...form.register("idealCustomer")}
              placeholder="Solo SaaS founders with an early product who struggle to turn content into qualified conversations…"
            />
          </Field>
          <p className="onboarding-reassurance">
            This is a hypothesis. Goalbar will help you test and improve it instead of treating it as fact.
          </p>
        </section>

        <section className="onboarding-section onboarding-section-compact" aria-labelledby="basics-heading">
          <div className="onboarding-section-heading">
            <span>3</span>
            <div>
              <p className="eyebrow">A few basics</p>
              <h2 id="basics-heading">What should we call you?</h2>
            </div>
          </div>
          <div className="field-grid two">
            <Field label="Your name" error={form.formState.errors.name?.message}>
              <Input autoComplete="name" {...form.register("name")} placeholder="Duc" />
            </Field>
            <Field label="Product or company" error={form.formState.errors.productName?.message}>
              <Input autoComplete="organization" {...form.register("productName")} placeholder="Goalbar" />
            </Field>
          </div>
          <details className="onboarding-optional">
            <summary>Add optional expertise or perspective</summary>
            <Field
              label="What do you know unusually well?"
              hint="Experience, hard-won lessons, or a strong point of view."
              error={form.formState.errors.expertise?.message}
            >
              <Textarea
                rows={3}
                {...form.register("expertise")}
                placeholder="I have spent the last five years building…"
              />
            </Field>
          </details>
        </section>

        {save.isError && (
          <div className="inline-error">
            <strong>Could not save</strong>
            <span>{save.error.message}</span>
          </div>
        )}
        <footer className="onboarding-actions">
          <div className="onboarding-privacy">
            <LockKeyhole size={15} aria-hidden="true" />
            <span>
              <strong>Private by default</strong>
              <small>Stored on this machine. Edit or reset it anytime.</small>
            </span>
          </div>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? "Creating your profile…" : "Create my starting profile"}
            <ArrowRight size={16} />
          </Button>
        </footer>
      </form>
    </div>
  )
}

function Field({
  label,
  hint,
  error,
  icon,
  children,
}: {
  label: string
  hint?: string
  error?: string
  icon?: React.ReactNode
  children: ReactElement<{
    id?: string
    "aria-describedby"?: string
    "aria-invalid"?: boolean
  }>
}) {
  const generatedId = useId()
  const controlId = children.props.id ?? generatedId
  const hintId = hint ? `${generatedId}-hint` : undefined
  const errorId = error ? `${generatedId}-error` : undefined
  const describedBy = [children.props["aria-describedby"], hintId, errorId].filter(Boolean).join(" ")
  const control = cloneElement(children, {
    id: controlId,
    "aria-describedby": describedBy || undefined,
    "aria-invalid": error ? true : children.props["aria-invalid"],
  })

  return (
    <div className="field">
      <label className="field-label" htmlFor={controlId}>
        {icon}
        {label}
      </label>
      {hint && <small id={hintId}>{hint}</small>}
      {control}
      {error && <em id={errorId}>{error}</em>}
    </div>
  )
}
