import { useEffect } from "react"

/**
 * Hook that triggers a callback when the Escape key is pressed while some state is active (e.g. popover open).
 */
export const useEscapeKey = (isOpen: boolean, onClose: () => void) => {
    useEffect(() => {
        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === "Escape" && isOpen) {
                onClose()
            }
        }

        if (isOpen) {
            document.addEventListener("keydown", handleEscape)
        }

        return () => {
            document.removeEventListener("keydown", handleEscape)
        }
    }, [isOpen, onClose])
}
