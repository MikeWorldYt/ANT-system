import { Component, OnInit } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';
import { RouterOutlet } from '@angular/router';

// ▲ SERVICES ▲
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';
import { LanguageService } from '../../../services/navLanguage.service';

@Component({
  selector: 'docs-t02-into-main-concepts',
  standalone: true,
  imports: [
    FooterComponent,
    RouterOutlet
  ],
  templateUrl: './into-main-concepts.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_IntoComponent implements OnInit {
  currTitle: string = 'introduction';

  constructor(
    private languageService: LanguageService,
    private TitleService: TitleService,
    private pageService: PageService,
  ) {}

  ngOnInit(): void {
    // Set TitleValue Service
    this.TitleService.setTitle('title_02');
  }

  toggle(section: string) {
    this.currTitle = section;
  }

}
