---
name: Share a suggestion
about: Suggest an improvement, new feature, or something we could do better
labels: ''
body:
  - type: markdown
    id: preface
    attributes:
      value: "Thank you for submitting a suggestion for Spin!"
  - type: input
    id: version
    attributes:
      label: "What is the version of your Spin CLI?"
      description: "You can use te command `spin --version` to get it."
  - type: textarea
    id: description
    attributes:
      label: "What is your suggestion?"
      validations:
        required: true
  - type: textarea
    id: "usecase"
    attributes:
      label: "Why would this improve Spin?"
      description: "Please describe your use case or scenario."
    validations:
      required: true
  - type: checkboxes
    id: contributing
    attributes:
      label: "Are you willing to submit PRs to contribute to this feature or improvement?"
      description: "This is not required and we are happy to guide you in the contribution process."
      options:
        - label: Yes, I am willing to implement it.
---

