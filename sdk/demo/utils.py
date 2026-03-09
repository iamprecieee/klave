try:
    from langchain_core.callbacks import BaseCallbackHandler
except ImportError:
    LANGCHAIN_AVAILABLE = False


SOL_MINT = "So11111111111111111111111111111111111111112"
USDC_MINT = "2VyjXZnCMDRNe32DBvA6TPdrRgCUjfMmsTajtgTgEvWf"  # Devnet USDC
CREAM, GREY, AMBER, BLUE, TEAL = 255, 242, 172, 75, 36


def _color(code: int, text: str) -> str:
    return f"\033[38;5;{code}m{text}\033[0m"


def _status(text: str, level: str = "info") -> None:
    icon = {"info": "◇", "warn": "⚠", "err": "✖"}.get(level, "◇")
    color = {"info": TEAL, "warn": AMBER, "err": 1}.get(level, TEAL)
    print(f"{_color(color, icon)}  {_color(CREAM, text)}")


def flow_start(cmd: str) -> None:
    print(f"{_color(AMBER, '┌')}  {_color(CREAM, f'klave agent — {cmd}')}")


def flow_step(text: str) -> None:
    print(f"{_color(TEAL, '◇')}  {_color(CREAM, text)}")


def flow_line(text: str) -> None:
    print(f"{_color(AMBER, '│')}  {text}")


def flow_thought(text: str) -> None:
    print(
        f"{_color(AMBER, '│')}  {_color(GREY, '💭 [LLM THINKING]')} {_color(GREY, text)}"
    )


def flow_done(text: str) -> None:
    print(f"{_color(AMBER, '│')}  {_color(TEAL, '◆')} {_color(CREAM, text)}")


def flow_end(text: str) -> None:
    print(f"{_color(AMBER, '└')}  {_color(CREAM, text)}")


class FlowCallbackHandler(BaseCallbackHandler):
    """Custom LangChain callback to match the KLAVE CLI aesthetic."""

    def on_llm_start(self, serialized, prompts, **kwargs) -> None:
        pass  # We use flow_step manually for session start

    def on_agent_action(self, action, **kwargs) -> None:
        flow_thought(f"Invoking `{action.tool}` with `{action.tool_input}`")

    def on_tool_end(self, output, **kwargs) -> None:
        output_str = (
            str(output)[:100] + "..." if len(str(output)) > 100 else str(output)
        )
        flow_done(f"Tool Result: {output_str}")

    def on_agent_finish(self, finish, **kwargs) -> None:
        pass  # The final answer is handled in main to apply flow_done
