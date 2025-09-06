import asyncio
from shinka.core import AsyncEvolutionRunner, EvolutionConfig
from shinka.launch import LocalJobConfig
from shinka.database import DatabaseConfig

TASK_SYS_MST = open("prompt.md").read().strip()

async def main():
    evo_config = EvolutionConfig(
        task_sys_msg=TASK_SYS_MST,
        init_program_path="initial.rs",
        language="rust",
        patch_types=["full", "diff", "cross"],
        patch_type_probs=[0.45, 0.45, 0.1],
        num_generations=50,
        max_parallel_jobs=20,
        llm_models=["gpt-5"],
        llm_kwargs=dict(
            temperatures=[1.0],
            max_tokens=128000,  #16384,
            reasoning_efforts=["high"],
        ),
    )
    
    runner = AsyncEvolutionRunner(
        evo_config=evo_config,
        job_config=LocalJobConfig(
            eval_program_path="evaluate.py",
        ),
        db_config=DatabaseConfig(db_path="evolution.sqlite"),
        max_proposal_jobs=10,  # 10,  # Generate 10 proposals concurrently
        max_evaluation_jobs=1,  # 0,  # Proposals to evaluate in parallel
    )
    
    await runner.run()

asyncio.run(main())