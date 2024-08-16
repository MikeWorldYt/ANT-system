import { Component, OnInit } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';
import { Docs_T02_Mod01_Component } from '../mod-01-levels-structure/mod-01-levels-structure.component';
import { Docs_T02_Mod02_Component } from '../mod-02-support-folders/mod-02-support-folders.component';
import { Docs_T02_Mod03_Component } from '../mod-03-nomenclature/mod-03-nomenclature.component';
import { TitleStateService } from '../../../services/title.service';

@Component({
  selector: 'docs-t02-into-main-concepts',
  standalone: true,
  imports: [
    FooterComponent,
    Docs_T02_Mod01_Component,
    Docs_T02_Mod02_Component,
    Docs_T02_Mod03_Component
  ],
  templateUrl: './into-main-concepts.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_IntoComponent implements OnInit {
  currentSection: string = 'introduction';

  constructor(
    private titleStateService: TitleStateService
  ) {}

  ngOnInit(): void {
    // Set title section
    this.titleStateService.setCurrentSection('concepts');
  }

  toggle(section: string) {
    this.currentSection = section;
  }

}
