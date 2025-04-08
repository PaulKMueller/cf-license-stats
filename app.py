import streamlit as st
import json
import plotly.express as px
import pandas as pd

st.set_page_config(
    page_title="License Distribution",
    page_icon=":bar_chart:",
)

with open("sorted_license_counter.json") as f:
    data = json.load(f)

df = pd.DataFrame(data)

df = df.nlargest(14, "count")

df.head()

st.title("License Distribution")

repo_url = "https://github.com/PaulKMueller/cf-license-stats"
icon_url = "https://github.githubassets.com/images/modules/logos_page/GitHub-Mark.png"

st.markdown(
    f"""
    <a href="{repo_url}" target="_blank">
        <button style='z-index: 1000; margin: 1rem; position: absolute; top: 0; right: 0; border: none; color: white; background-color: black; border-radius: 5px; padding: 10px 20px;'>
            <img src="{icon_url}" alt="GitHub" style="height: 20px; width: 20px; margin-right: 5px;">
            GitHub
        </button>
    </a>
    """,
    unsafe_allow_html=True,
)

fig = px.pie(df, values="count", names="license", title="License Distribution")
st.plotly_chart(fig, use_container_width=True)
st.markdown(
    "`INVALID` indicates, that the license of a package did not comply with [SPDX formatting](https://spdx.org/licenses/)."
)
