# ZapFast Business CRM workspace

This document records the product direction and the account-isolation rule for
the Business-only CRM features. It complements `BUSINESS-MAINTENANCE.md` and
keeps future upstream merges small.

## Implemented foundation after 0.19.11

- Account-scoped persistent CRM tasks stored in the encrypted archive.
- Task title, description, priority, due date, completion state and optional chat link.
- Create, edit, complete/reopen, delete and open the linked conversation from the task center.
- CRM backup version 2 includes tasks and still accepts older backups without a `tasks` field.
- The task model is the shared base for the planned agenda and project views.
- The task center now has My day, task list and responsive monthly agenda views.
- Task deadlines use an exact local calendar date and time instead of elapsed-hour shortcuts.

## What exists in 0.19.10

- A resizable CRM sidecar for pipeline stage, deal value, tags, encrypted
  internal notes and follow-up reminders.
- A visual sales funnel with custom stages, contact cards, search, backup and
  metrics.
- Desktop reminders for due follow-ups.
- Business quick replies (text and voice), scheduled messages, broadcast lists
  and bulk dispatch.
- More than one linked WhatsApp account in the same window.

## Account isolation invariant

Every value derived from or created for a WhatsApp account must live in that
account's `AccountDirs` archive and in the corresponding in-memory `Account`.
This includes CRM stages, deals, notes, tags, follow-ups, quick replies,
scheduled messages, broadcast lists, tasks, projects and calendar events.

The UI may keep only presentation state globally (for example whether a modal
is open). It must never keep business records globally. Backend events from an
inactive account are applied to that account and must not replace the records
shown for the active account.

## Product structure

### Work inbox

The bell in the chat-list header opens one account-scoped inbox. It starts with
follow-ups and should evolve to combine:

- overdue, today and upcoming follow-ups;
- assigned CRM tasks and their priority;
- scheduled messages and failed sends;
- upcoming meetings;
- one-click actions to open the related conversation, complete or postpone.

### Tasks

The next schema should add `crm_tasks` with an immutable id, optional chat,
optional project, title, description, status, priority, due time, completion
time and timestamps. A task may exist without a WhatsApp contact. Deleting a
contact or removing a deal from the funnel must not silently delete tasks.

### Projects

`crm_projects` should represent delivery after a sale. A project can link one
or more WhatsApp chats and contain:

- client and responsible person;
- status, start date, deadline and commercial value;
- tasks, notes, links/access records and deliverables;
- paid-traffic summary and external dashboard links.

Secrets and passwords must not be stored as plain project text. External
credentials need a dedicated Windows credential-store integration.

### Calendar

`crm_events` should support meetings, calls, deadlines and follow-ups with
start/end time, local timezone, location or meeting URL, notes, recurrence and
links to a chat/project. The first version is a local agenda with day, week and
month views. Calendar-provider synchronization should be a later adapter so the
local CRM remains usable offline and no provider tokens enter the WhatsApp
archive accidentally.

### Fluxo integration

The safe integration boundary is an explicit adapter, not direct reuse of the
Fluxo database. Define a versioned API for creating a project from a WhatsApp
conversation and returning the external project id/URL. Map only fields the
user chose to send. Never upload message history by default.

The current Fluxo interface was reviewed in read-only mode on 9 October 2026.
The ZapFast Business design should reuse these proven concepts without copying
Fluxo data or credentials:

- project cards with status, task completion ratio, percentage and deadline;
- task views as Kanban, list, checklist and calendar, with project/status
  filters, priority, assignee and timer;
- a project workspace with Kanban, client dossier, credentials, deliverables,
  documents and paid-traffic tabs;
- month/week agenda combining customer-success events, meetings, tasks, sales
  calls and paid-traffic end dates;
- a personal "My day" view as the default operational inbox.

In ZapFast Business, the first release of this pattern is the account-scoped
work inbox. Later tables and screens should keep the same hierarchy:
`WhatsApp account -> contact/deal -> project -> task/event`.

## Delivery order

1. Keep CRM, quick replies and reminders isolated per linked WhatsApp account.
2. Expand the work inbox and add CRUD for tasks. **Completed.**
3. Add projects and link them to contacts/deals.
4. Add local calendar views and meeting/event CRUD. **Task and follow-up calendar delivered; meeting/event records remain.**
5. Add optional Fluxo and calendar-provider adapters with explicit credentials,
   field mapping and audit logs.
6. Add reports, assignment and team synchronization after the local data model
   is stable.

Each phase needs archive migration tests, a two-account isolation test, a
restart/persistence test and a narrow-window layout check before release.
